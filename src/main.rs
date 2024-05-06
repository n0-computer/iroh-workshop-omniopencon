use clap::Parser;
use iroh_net::{key::SecretKey, magic_endpoint, ticket::NodeTicket, MagicEndpoint};
use tracing::info;
mod util;
use util::*;

/// The ALPN we use for this protocol.
const PIPE_ALPN: &[u8] = b"JOTB_PIPE";

#[derive(Debug, clap::Parser)]
struct Args {
    /// Ticket to connect to. If not provided, the program will listen for incoming connections.
    ticket: Option<NodeTicket>,
}

/// Connect to a remote node using a ticket.
async fn connect(ticket: NodeTicket) -> anyhow::Result<()> {
    let secret_key = SecretKey::generate();
    let public_key = secret_key.public();
    // Create a new MagicEndpoint with the secret key.
    // We bind to port 0 to let the OS choose a random port.
    let endpoint = MagicEndpoint::builder()
        .secret_key(secret_key)
        .bind(0)
        .await?;
    let addr = ticket.node_addr().clone();
    info!("connecting to {:?}", addr);
    let connection = endpoint.connect(addr, PIPE_ALPN).await?;
    let (mut send, recv) = connection.open_bi().await?;
    tracing::info!("opened bidirectional stream");
    tracing::info!("copying from stdin to remote");
    let remote_node_id = magic_endpoint::get_remote_node_id(&connection)?;
    let remote = remote_node_id.to_string();
    send.write_all(format!("hello from {}\n", public_key).as_bytes())
        .await?;
    tokio::spawn(copy_to_stdout(remote, recv));
    copy_stdin_to(send).await?;
    Ok(())
}

/// Accept incoming connections.
async fn accept() -> anyhow::Result<()> {
    let secret_key = SecretKey::generate();
    let public_key = secret_key.public();
    let endpoint = MagicEndpoint::builder()
        .secret_key(secret_key)
        .alpns(vec![PIPE_ALPN.to_vec()])
        .bind(0)
        .await?;
    let addr = endpoint.my_addr().await?;
    println!("I am {}", addr.node_id);
    println!("Listening on {:#?}", addr.info);
    println!("ticket: {}", NodeTicket::new(addr)?);
    while let Some(connecting) = endpoint.accept().await {
        // handle each incoming connection in separate tasks.
        todo!();
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // init logging. we can now configure the log level with the RUST_LOG environment variable.
    tracing_subscriber::fmt::init();
    // Parse the command line arguments.
    let args = Args::parse();
    // if a ticket is provided, connect to the remote node, otherwise accept incoming connections.
    if let Some(ticket) = args.ticket {
        connect(ticket).await?;
    } else {
        accept().await?;
    }
    Ok(())
}
