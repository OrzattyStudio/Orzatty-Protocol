use quinn::Endpoint;
use orzatty_core::frame::{FrameHeader, FrameType, FrameFlags};
use orzatty_core::Framer;
use orzatty_core::auth::AuthMessage;
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let addr: SocketAddr = "127.0.0.1:5000".parse()?;
    
    // 💡 Generate a simple self-signed cert for the example
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let cert_der = cert.serialize_der()?;
    let priv_key = cert.serialize_private_key_der();
    
    let server_config = quinn::ServerConfig::with_single_cert(
        vec![rustls::Certificate(cert_der)],
        rustls::PrivateKey(priv_key)
    )?;

    let endpoint = Endpoint::server(server_config, addr)?;
    println!("🦅 Orzatty Reference Server listening on {}", addr);

    while let Some(conn) = endpoint.accept().await {
        tokio::spawn(async move {
            if let Err(e) = handle_client(conn).await {
                eprintln!("❌ Client error: {:?}", e);
            }
        });
    }

    Ok(())
}

async fn handle_client(conn: quinn::Connecting) -> Result<()> {
    let connection = conn.await?;
    println!("🔗 New connection from {}", connection.remote_address());

    // 1. Simple Auth Handshake
    let (mut send, mut recv) = connection.accept_bi().await?;
    let mut framer = Framer::new();
    
    if let Some((_header, payload)) = framer.read_frame(&mut recv).await? {
        let auth: AuthMessage = rkyv::from_bytes(&payload)
            .map_err(|_| anyhow::anyhow!("Failed to deserialize auth message"))?;
            
        match auth {
            AuthMessage::Hello { token } => {
                println!("🔑 Auth attempt with token: {}", token);
                // In this basic version, we accept everything
                let resp = rkyv::to_bytes::<_, 32>(&AuthMessage::Ok)
                    .map_err(|_| anyhow::anyhow!("Failed to serialize auth response"))?;
                let head = FrameHeader {
                    flags: FrameFlags::empty(),
                    frame_type: FrameType::RkyvAligned,
                    channel_id: 0,
                    stream_id: 0,
                    length: resp.len() as u64,
                };
                let mut buf = [0u8; 32];
                let n = head.encode(&mut buf)?;
                send.write_all(&buf[..n]).await?;
                send.write_all(&resp).await?;
                println!("✅ Client Authenticated.");
            }
            _ => return Err(anyhow::anyhow!("Expected Auth Hello")),
        }
    }

    // 2. Simple Echo Loop
    loop {
        let (mut send, mut recv) = connection.accept_bi().await?;
        tokio::spawn(async move {
            let mut framer = Framer::new();
            while let Some((header, payload)) = framer.read_frame(&mut recv).await.unwrap_or(None) {
                println!("📥 Received {} bytes on channel {}", payload.len(), header.channel_id);
                // Echo back
                let mut buf = [0u8; 32];
                let n = header.encode(&mut buf).unwrap();
                let _ = send.write_all(&buf[..n]).await;
                let _ = send.write_all(&payload).await;
            }
        });
    }
}
