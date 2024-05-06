use clap::Parser;
use iroh_base::node_addr::AddrInfoOptions;
use iroh_gossip::proto::TopicId;
use iroh_net::{ticket::NodeTicket, MagicEndpoint};

mod util;
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
    let public_key = secret_key.public();
    let topic = TopicId::from([0u8; 32]);

    let endpoint = MagicEndpoint::builder()
        .secret_key(secret_key.clone())
        .alpns(vec![iroh_gossip::net::GOSSIP_ALPN.to_vec()])
        .bind(0)
        .await?;
    let mut my_addr = endpoint.my_addr().await?;
    let ticket = NodeTicket::new(my_addr.clone())?;
    println!("I am {}", my_addr.node_id);
    println!("Connect to me using {}", ticket);
    my_addr.apply_options(AddrInfoOptions::Id);
    let short = NodeTicket::new(my_addr.clone())?;
    println!("Connect to me using {}", short);
    wait_for_relay(&endpoint).await?;
    // chat goes here
    Ok(())
}
