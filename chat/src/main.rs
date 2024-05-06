use clap::Parser;
use iroh_net::ticket::NodeTicket;

#[derive(Debug, Parser)]
struct Args {
    tickets: Vec<NodeTicket>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    // chat goes here
    Ok(())
}
