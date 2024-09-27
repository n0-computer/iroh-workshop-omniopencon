use clap::Parser;
use iroh_net::{
    endpoint::{self, get_remote_node_id},
    key::{PublicKey, SecretKey},
    ticket::NodeTicket,
    Endpoint,
};
use tracing::info;
mod util;
use util::*;

/// The ALPN we use for this protocol.
const PIPE_ALPN: &[u8] = b"PIPE";

#[derive(Debug, clap::Parser)]
struct Args {
    /// Ticket to connect to. If not provided, the program will listen for incoming connections.
    ticket: Option<NodeTicket>,
}

/// Connect to a remote node using a ticket.
async fn connect(ticket: NodeTicket) -> anyhow::Result<()> {
    let secret_key = SecretKey::generate();
    let public_key = secret_key.public();
    // Create a new Endpoint with the secret key.
    // We bind to port 0 to let the OS choose a random port.
    let endpoint = Endpoint::builder().secret_key(secret_key).bind().await?;
    let addr = ticket.node_addr().clone();
    info!("connecting to {:?}", addr);
    let connection = endpoint.connect(addr, PIPE_ALPN).await?;
    let (mut send, recv) = connection.open_bi().await?;
    tracing::info!("opened bidirectional stream");
    tracing::info!("copying from stdin to remote");
    let remote_node_id = endpoint::get_remote_node_id(&connection)?;
    let remote = remote_node_id.to_string();
    send.write_all(format!("hello from {}\n", public_key).as_bytes())
        .await?;
    tokio::spawn(copy_to_stdout(remote, recv));
    copy_stdin_to(send).await?;
    Ok(())
}

/// Handle a single incoming connection.
async fn handle_incoming(
    my_id: &PublicKey,
    incoming: iroh_net::endpoint::Incoming,
) -> anyhow::Result<()> {
    info!("connection attempt");
    // accept the connection and get the ALPN and the bidirectional stream.
    let mut connecting = incoming.accept()?;
    let alpn: Vec<u8> = connecting.alpn().await?;
    let connection = connecting.await?;
    let remote_node_id = get_remote_node_id(&connection)?;
    info!(
        "got connection from {} using ALPN {:?}",
        remote_node_id, alpn
    );
    // check if the ALPN is what we expect.
    if alpn.as_slice() != PIPE_ALPN {
        tracing::warn!("unexpected ALPN: {:?}", alpn);
        return Ok(());
    }
    // we have already accepted the connection, but we need to accept a stream on the connection.
    let (mut send, recv) = connection.accept_bi().await?;
    info!("accepted bidirectional stream");
    info!("copying from stdin to remote");
    let author = remote_node_id.to_string();
    // Send a greeting to the remote node.
    send.write_all(format!("hello from {}\n", my_id).as_bytes())
        .await?;
    // Spawn two tasks to copy data in both directions.
    tokio::spawn(copy_stdin_to(send));
    tokio::spawn(copy_to_stdout(author, recv));
    // this will return immediately, the tasks will keep running in the background.
    Ok(())
}

/// Accept incoming connections.
async fn accept() -> anyhow::Result<()> {
    let secret_key = get_or_create_secret()?;
    let public_key = secret_key.public();
    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        .alpns(vec![PIPE_ALPN.to_vec()])
        .bind()
        .await?;
    wait_for_relay(&endpoint).await?;
    let addr = endpoint.node_addr().await?;
    println!("I am {}", addr.node_id);
    println!("Listening on {:#?}", addr.info);
    println!(
        "Connect to me using\ncargo run {}\n",
        NodeTicket::new(addr)?
    );
    while let Some(incoming) = endpoint.accept().await {
        // handle each connection sequentially.
        if let Err(cause) = handle_incoming(&public_key, incoming).await {
            tracing::warn!("error handling connection: {:?}", cause);
        }
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
