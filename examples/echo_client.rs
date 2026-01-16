use orzatty_client::OrzattyClient;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 🔗 1. Create the indestructible client
    println!("🚀 Orzatty Echo Client Starting...");
    let client = OrzattyClient::new().await?;

    // 📍 2. Define server address and token
    let addr: SocketAddr = "127.0.0.1:5000".parse()?;
    let token = "YOUR_SECRET_TOKEN"; // Change this in production

    // 🤝 3. Connect to the server
    println!("🔗 Connecting to {}...", addr);
    let connection = client.connect(addr, "localhost", token).await?;
    println!("✅ Authenticated and Connected!");

    // 📤 4. Send a message on Channel 1
    let message = b"Hello from the open-source client!";
    println!("📤 Sending: {:?}", String::from_utf8_lossy(message));
    
    // The .send() method uses actor-based backpressure (The Governor)
    connection.send(1, message).await?;

    println!("🎉 Message sent successfully! Orzatty is reliable and fast.");

    Ok(())
}
