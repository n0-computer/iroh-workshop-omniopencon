use clap::Parser;
use iroh_base::node_addr::AddrInfoOptions;
use iroh_gossip::{
    net::{Event, Gossip},
    proto::TopicId,
};
use iroh_net::{
    discovery::{dns::DnsDiscovery, pkarr_publish::PkarrPublisher, ConcurrentDiscovery},
    magic_endpoint,
    ticket::NodeTicket,
    MagicEndpoint,
};

mod util;
use tokio::io::{AsyncBufReadExt, BufReader};
use util::*;

#[derive(Debug, Parser)]
struct Args {
    tickets: Vec<NodeTicket>,
}

/// Handle incoming connections by dispatching them to the right handler.
async fn handle_connections(endpoint: MagicEndpoint, gossip: Gossip) -> anyhow::Result<()> {
    while let Some(connecting) = endpoint.accept().await {
        let gossip = gossip.clone();
        tokio::spawn(async move {
            let (_, alpn, connection) = magic_endpoint::accept_conn(connecting).await?;
            if alpn.as_bytes() == iroh_gossip::net::GOSSIP_ALPN {
                gossip.handle_connection(connection).await?;
            }
            anyhow::Ok(())
        });
    }
    Ok(())
}

/// Print messages from the gossip stream to stdout.
async fn print_messages(gossip: Gossip, topic: TopicId) -> anyhow::Result<()> {
    let mut stream = gossip.subscribe(topic).await?;
    while let Ok(event) = stream.recv().await {
        match event {
            Event::Received(ev) => {
                let text = String::from_utf8_lossy(&ev.content);
                tracing::info!("received message: {}", text);
                println!("message {}", text);
            }
            ev => {
                tracing::info!("event {:?}", ev);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let secret_key = get_or_create_secret()?;
    let public_key = secret_key.public();
    let topic = TopicId::from([0u8; 32]);
    let discovery = Box::new(ConcurrentDiscovery::from_services(vec![
        Box::new(DnsDiscovery::n0_dns()),
        Box::new(PkarrPublisher::n0_dns(secret_key.clone())),
    ]));

    let endpoint = MagicEndpoint::builder()
        .secret_key(secret_key.clone())
        .alpns(vec![iroh_gossip::net::GOSSIP_ALPN.to_vec()])
        .discovery(discovery)
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
    // add all the info from the tickets to the endpoint
    let mut ids = Vec::new();
    for ticket in &args.tickets {
        let addr = ticket.node_addr();
        endpoint.add_node_addr(addr.clone())?;
        ids.push(addr.node_id);
    }
    let gossip = Gossip::from_endpoint(
        endpoint.clone(),
        iroh_gossip::proto::Config::default(),
        &my_addr.info,
    );
    tokio::spawn(handle_connections(endpoint, gossip.clone()));
    tokio::spawn(print_messages(gossip.clone(), topic));
    println!("joining topic {}", topic);
    gossip.join(topic, ids).await?.await?;
    println!("joined topic");
    let mut stdin = BufReader::new(tokio::io::stdin()).lines();
    while let Some(line) = stdin.next_line().await? {
        gossip.broadcast(topic, line.into_bytes().into()).await?;
    }
    Ok(())
}
