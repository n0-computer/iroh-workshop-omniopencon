use clap::Parser;
use iroh_net::{key::SecretKey, ticket::NodeTicket, MagicEndpoint};
use tracing::info;

/// The ALPN we use for this protocol.
const PIPE_ALPN: &[u8] = b"JOTB_PIPE";

#[derive(Debug, clap::Parser)]
struct Args {
    /// Ticket to connect to. If not provided, the program will listen for incoming connections.
    ticket: NodeTicket,
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
    todo!()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // init logging. we can now configure the log level with the RUST_LOG environment variable.
    tracing_subscriber::fmt::init();
    // Parse the command line arguments.
    let args = Args::parse();
    // Code goes here
    connect(args.ticket).await?;
    Ok(())
}
