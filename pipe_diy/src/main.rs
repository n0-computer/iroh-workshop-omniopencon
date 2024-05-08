#![allow(unused_imports, unused_variables, dead_code)]
use clap::Parser;
use iroh_net::{
    key::{PublicKey, SecretKey},
    magic_endpoint,
    ticket::NodeTicket,
    MagicEndpoint,
};
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // init logging. we can now configure the log level with the RUST_LOG environment variable.
    tracing_subscriber::fmt::init();
    // Parse the command line arguments.
    let args = Args::parse();
    todo!("draw the rest of the owl");
}
