use clap::Parser;
use futures::StreamExt;
use iroh_base::node_addr::AddrInfoOptions;
use iroh_gossip::{net::Gossip, proto::TopicId};
use iroh_net::{
    discovery::{dns::DnsDiscovery, pkarr::PkarrPublisher, ConcurrentDiscovery},
    ticket::NodeTicket,
    Endpoint,
};

mod util;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    select,
};
use util::*;

#[derive(Debug, Parser)]
struct Args {
    tickets: Vec<NodeTicket>,
}

/// Handle incoming connections by dispatching them to the right handler.
async fn handle_connections(endpoint: Endpoint, gossip: Gossip) -> anyhow::Result<()> {
    while let Some(incoming) = endpoint.accept().await {
        let gossip = gossip.clone();
        tokio::spawn(async move {
            let mut connecting = incoming.accept()?;
            let alpn = connecting.alpn().await?;
            let connection = connecting.await?;
            if &alpn == iroh_gossip::net::GOSSIP_ALPN {
                gossip.handle_connection(connection).await?;
            }
            anyhow::Ok(())
        });
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let secret_key = get_or_create_secret()?;
    let _public_key = secret_key.public();
    let topic = TopicId::from([0u8; 32]);
    let discovery = Box::new(ConcurrentDiscovery::from_services(vec![
        Box::new(DnsDiscovery::n0_dns()),
        Box::new(PkarrPublisher::n0_dns(secret_key.clone())),
    ]));

    let endpoint = Endpoint::builder()
        .secret_key(secret_key.clone())
        .alpns(vec![iroh_gossip::net::GOSSIP_ALPN.to_vec()])
        .discovery(discovery)
        .bind()
        .await?;
    let mut my_addr = endpoint.node_addr().await?;
    let ticket = NodeTicket::new(my_addr.clone())?;
    println!("I am {}", my_addr.node_id);
    println!("Connect to me using {}", ticket);
    my_addr.apply_options(AddrInfoOptions::Id);
    let short = NodeTicket::new(my_addr.clone())?;
    println!("Connect to me using {}", short);
    wait_for_relay(&endpoint).await?;
    // add all the info from the tickets to the endpoint
    let mut ids = Vec::new();
    for ticket in &args.tickets {
        let addr = ticket.node_addr();
        endpoint.add_node_addr(addr.clone()).ok();
        ids.push(addr.node_id);
    }
    let gossip = Gossip::from_endpoint(
        endpoint.clone(),
        iroh_gossip::proto::Config::default(),
        &my_addr.info,
    );
    tokio::spawn(handle_connections(endpoint, gossip.clone()));
    let mut gossip = gossip.join(topic, ids).await?;
    let mut stdin = BufReader::new(tokio::io::stdin()).lines();
    loop {
        select! {
            message = gossip.next() => {
                if let Some(Ok(event)) = message {
                    println!("{:?}", event);
                } else {
                    break;
                }
            }
            line = stdin.next_line() => {
                if let Ok(Some(line)) = line {
                    gossip.broadcast(line.into_bytes().into()).await?;
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}
