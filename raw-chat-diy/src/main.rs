#![allow(unused_imports, unused_variables, dead_code)]
use clap::Parser;
use iroh_base::node_addr::AddrInfoOptions;
use iroh_gossip::{
    net::{Event, Gossip},
    proto::TopicId,
};
use iroh_net::{
    discovery::{dns::DnsDiscovery, pkarr::PkarrPublisher, ConcurrentDiscovery},
    endpoint,
    ticket::NodeTicket,
    Endpoint,
};

mod util;
use tokio::io::{AsyncBufReadExt, BufReader};
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
