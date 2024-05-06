use std::str::FromStr;

use iroh_net::{
    key::{PublicKey, SecretKey},
    MagicEndpoint,
};
use tokio::io::{AsyncBufReadExt, BufReader};

// Copy from the remote to stdout, prepending the author's name.
pub async fn copy_to_stdout(author: String, from: quinn::RecvStream) -> anyhow::Result<()> {
    let mut lines = BufReader::new(from).lines();
    while let Some(line) = lines.next_line().await? {
        tracing::info!("read line: {}", line);
        println!("{}> {}", author, line);
    }
    Ok(())
}

// Copy from stdin to the remote.
pub async fn copy_stdin_to(mut to: quinn::SendStream) -> anyhow::Result<()> {
    let from = tokio::io::stdin();
    let mut lines = BufReader::new(from).lines();
    while let Some(line) = lines.next_line().await? {
        tracing::info!("read line: {}", line);
        to.write_all(format!("{}\n", line).as_bytes()).await?;
    }
    Ok(())
}

// Wait for the endpoint to figure out its relay address.
pub async fn wait_for_relay(endpoint: &MagicEndpoint) -> anyhow::Result<()> {
    while endpoint.my_relay().is_none() {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    Ok(())
}

/// Get the secret key from a file or generate a new one.
pub fn get_or_create_secret() -> anyhow::Result<SecretKey> {
    if let Ok(secret) = std::env::var("SECRET") {
        let secret = SecretKey::from_str(&secret)?;
        Ok(secret)
    } else {
        // Generate a new secret key and print it to the console.
        // DON'T DO THIS IN PRODUCTION!
        let secret = SecretKey::generate();
        println!("Using SECRET={}", secret);
        println!("To keep the node id stable, set the SECRET environment variable");
        Ok(secret)
    }
}

/// Print public key (aka node id) as a z32 string, compatible with https://pkarr.org/
pub fn z32_node_id(node_id: &PublicKey) -> String {
    zbase32::encode_full_bytes(node_id.as_bytes().as_slice())
}
