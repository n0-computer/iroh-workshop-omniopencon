#![allow(unused_imports, unused_variables, dead_code)]
use clap::Parser;
mod util;
use iroh::net::ticket::NodeTicket;
use util::*;

#[derive(Debug, Parser)]
struct Args {
    tickets: Vec<NodeTicket>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let secret_key = get_or_create_secret()?;
    todo!("draw the rest of the owl");
}
