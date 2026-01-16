use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use crate::OrzattyClient;
use orzatty_core::frame::{FrameHeader, FrameType, FrameFlags};
use orzatty_core::Framer;
use anyhow::{Result, anyhow};
use quinn::{Connection, SendStream};

/// A high-level wrapper around `OrzattyClient` that manages channels and callbacks.
/// 
/// # "The Governor" Architecture (Phase 2)
/// Implements the Actor Model for flow control.
/// - **App Layer:** Pushes messages to a bounded channel (`mpsc::Sender`). fast and non-blocking until buffer fills.
/// - **Network Layer:** A single background task owns the `SendStream` and drains the channel.
/// - **Backpressure:** If the network is slow, the channel fills up, and `send().await` naturally slows down the app.
#[derive(Clone)]
pub struct EasyClient {
    connection: Connection,
    router: Arc<Mutex<Router>>,
    // The "Governor" channel - entry point for all outgoing messages
    tx: mpsc::Sender<OutboundMessage>,
}

struct OutboundMessage {
    channel_id: u32,
    data: Vec<u8>,
}

type MsgCallback = Box<dyn Fn(Vec<u8>) + Send + Sync>;

struct Router {
    handlers: HashMap<u32, MsgCallback>,
    default_handler: Option<MsgCallback>,
}

impl EasyClient {
    pub async fn connect(addr: &str, token: &str) -> Result<Self> {
        let client = OrzattyClient::new().await?; 
        
        let socket_addr = addr.parse()
            .map_err(|_| anyhow!("Invalid address format"))?;

        let connection = client.connect(socket_addr, "localhost", token).await?;

        let router = Arc::new(Mutex::new(Router {
            handlers: HashMap::new(),
            default_handler: None,
        }));

        // Create the Governor Channel (Bounded for Backpressure)
        // Tune: Capacity = 64. 
        // Small buffer = Instant backpressure. Large buffer = Latency spikes.
        let (tx, rx) = mpsc::channel(64);

        // Configure Transport (Hardening)
        // Handled in OrzattyClient::new() now.
        
        let client = Self {
            connection: connection.clone(),
            router,
            tx,
        };

        // Initialize streams and spawn the Actor tasks
        client.init_system(rx).await?;

        Ok(client)
    }

    async fn init_system(&self, rx: mpsc::Receiver<OutboundMessage>) -> Result<()> {
        // Open a Bi-directional stream for the session
        let (send_stream, recv_stream) = self.connection.open_bi().await?;
        
        // 1. Spawn the "Writer Actor" (The Governor)
        // This task owns the SendStream exclusively. Zero contention.
        tokio::spawn(async move {
            Self::writer_loop(send_stream, rx).await;
        });

        // 2. Spawn the "Reader Actor"
        // This task owns the RecvStream exclusively.
        let router = self.router.clone();
        tokio::spawn(async move {
            Self::reader_loop(recv_stream, router).await;
        });

        Ok(())
    }

    /// The Writer Actor Loop
    /// Drains the queue and writes to the network as fast as possible.
    async fn writer_loop(mut stream: SendStream, mut rx: mpsc::Receiver<OutboundMessage>) {
        // Optimization: We could implement batching here if needed (read N items, write once).
        // For now, simple loop is already much faster than Mutex contention.
        
        // Reusable buffer for headers to avoid allocs
        let mut head_buf = [0u8; 32];

        while let Some(msg) = rx.recv().await {
            let header = FrameHeader {
                flags: FrameFlags::empty(),
                frame_type: FrameType::RawBinary, 
                channel_id: msg.channel_id,
                stream_id: stream.id().index(), 
                length: msg.data.len() as u64,
            };

            if let Ok(h_len) = header.encode(&mut head_buf) {
                // We ignore write errors here (if connection dies, loop will eventually exit)
                if stream.write_all(&head_buf[..h_len]).await.is_err() { break; }
                if stream.write_all(&msg.data).await.is_err() { break; }
            }
        }
        // Channel closed or write error
        let _ = stream.finish().await;
    }

    /// The Reader Actor Loop
    async fn reader_loop(mut stream: QuicRecvStream, router: Arc<Mutex<Router>>) {
        let mut framer = Framer::new();
        loop {
            match framer.read_frame(&mut stream).await {
                Ok(Some((header, payload))) => {
                    let router = router.lock().await;
                    if let Some(handler) = router.handlers.get(&header.channel_id) {
                        (handler)(payload.to_vec());
                    } else if let Some(default) = &router.default_handler {
                        (default)(payload.to_vec());
                    }
                }
                Ok(None) => break, // Stream closed
                Err(_) => break, // Error
            }
        }
    }

    pub async fn on(&self, channel_id: u32, callback: impl Fn(Vec<u8>) + Send + Sync + 'static) {
        let mut router = self.router.lock().await;
        router.handlers.insert(channel_id, Box::new(callback));
    }

    pub async fn on_any(&self, callback: impl Fn(Vec<u8>) + Send + Sync + 'static) {
        let mut router = self.router.lock().await;
        router.default_handler = Some(Box::new(callback));
    }

    pub async fn send(&self, channel_id: u32, data: &[u8]) -> Result<()> {
        // Send to the Governor channel.
        // If channel is full, this `.send().await` will pause (Backpressure).
        // This prevents the app from overwhelming the network buffer.
        self.tx.send(OutboundMessage {
            channel_id,
            data: data.to_vec(),
        }).await.map_err(|_| anyhow!("Connection closed (Governor dropped message)"))?;
        
        Ok(())
    }
}

// Type alias to make signagures cleaner
use quinn::RecvStream as QuicRecvStream;
