use anyhow::Result;
use quinn::{ClientConfig, Connection, Endpoint};
use std::{net::SocketAddr, sync::Arc};
use orzatty_core::frame::{FrameHeader, FrameType, FrameFlags};
use orzatty_core::auth::AuthMessage;
use orzatty_core::Framer;


pub mod easy; // Expose the new Easy API

pub struct OrzattyClient {
    endpoint: Endpoint,
}

impl OrzattyClient {
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    /// Creates a new Orzatty Client instance.
    /// Binds to 0.0.0.0:0 (random port) by default.
    /// 
    /// By default, this will:
    /// - Load system CA certificates for production use
    /// - Allow self-signed certificates if `ORZATTY_ALLOW_INSECURE=true` is set
    pub async fn new() -> Result<Self> {
        Self::with_config(true).await
    }

    /// Creates a new Orzatty Client with custom certificate validation.
    /// 
    /// # Arguments
    /// * `allow_insecure` - If true, allows self-signed certificates (dev only)
    pub async fn with_config(allow_insecure: bool) -> Result<Self> {
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates({
                let mut root_store = rustls::RootCertStore::empty();
                
                // Try to load system certificates
                if let Ok(certs) = rustls_native_certs::load_native_certs() {
                    for cert in certs {
                        let _ = root_store.add(&rustls::Certificate(cert.0));
                    }
                }
                
                root_store
            })
            .with_no_client_auth();

        // Allow self-signed certificates if requested (dev only)
        // Check environment variable or parameter
        let allow_self_signed = allow_insecure || 
            std::env::var("ORZATTY_ALLOW_INSECURE")
                .unwrap_or_else(|_| "false".to_string())
                .parse::<bool>()
                .unwrap_or(false);
        
        if allow_self_signed {
            client_crypto.dangerous().set_certificate_verifier(Arc::new(SkipServerVerification));
        }

        let mut client_config = ClientConfig::new(Arc::new(client_crypto));
        
        // Hardening: Increase timeouts and enable keep-alives
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_idle_timeout(Some(quinn::VarInt::from_u32(10_000).into())); // 10s
        transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(2)));
        client_config.transport_config(Arc::new(transport_config));

        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;
        endpoint.set_default_client_config(client_config);
        
        Ok(Self { endpoint })
    }

    /// Connects to an Orzatty Server and authenticates.
    pub async fn connect(&self, addr: SocketAddr, server_name: &str, token: &str) -> Result<Connection> {
        let connection = self.endpoint.connect(addr, server_name)?.await?;
        
        // --- Auth Handshake ---
        // Open bidirectional stream (Stream 0)
        let (mut send, mut recv) = connection.open_bi().await?;
        
        // 1. Send AuthHello
        let auth_msg = AuthMessage::Hello { token: token.to_string() };
        let auth_bytes = rkyv::to_bytes::<_, 256>(&auth_msg)
            .map_err(|e| anyhow::anyhow!("Failed to serialize auth hello: {:?}", e))?;
            
        let mut head_buf = [0u8; 32];
        let header = FrameHeader {
            flags: FrameFlags::empty(),
            frame_type: FrameType::RkyvAligned,
            channel_id: 0,
            stream_id: 0,
            length: auth_bytes.len() as u64,
        };
        let h_len = header.encode(&mut head_buf)?;
        
        send.write_all(&head_buf[..h_len]).await?;
        send.write_all(&auth_bytes).await?;
        send.finish().await?;
        
        // 2. Wait for AuthResponse using Framer
        let mut framer = Framer::new();
        let (_resp_header, payload) = framer.read_frame(&mut recv).await?
            .ok_or(anyhow::anyhow!("Server closed auth stream before response"))?;
        
        let resp_msg: AuthMessage = rkyv::from_bytes(&payload)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize auth response: {:?}", e))?;
            
        match resp_msg {
            AuthMessage::Ok => {
                // Return connection, ready to be used
                Ok(connection)
            }
            AuthMessage::Fail { reason } => {
                Err(anyhow::anyhow!("Authentication Failed: {}", reason))
            }
            _ => Err(anyhow::anyhow!("Unexpected auth response")),
        }
    }
}

// Internal helper for skipping cert verification in Dev mode
struct SkipServerVerification;

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}
