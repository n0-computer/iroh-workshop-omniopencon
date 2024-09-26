use clap::Parser;
use futures::StreamExt;
use iroh_base::node_addr::AddrInfoOptions;
use iroh_gossip::{
    net::{Event, Gossip, GossipEvent},
    proto::TopicId,
};
use iroh_net::{
    discovery::{dns::DnsDiscovery, pkarr::PkarrPublisher, ConcurrentDiscovery},
    key::{PublicKey, SecretKey, Signature},
    ticket::NodeTicket,
    Endpoint,
};

mod util;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    select,
};
use util::*;

#[derive(Debug, Parser)]
struct Args {
    tickets: Vec<NodeTicket>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SignedMessage {
    from: PublicKey,
    data: Vec<u8>,
    signature: Signature,
}

impl SignedMessage {
    pub fn verify_and_decode(bytes: &[u8]) -> anyhow::Result<(PublicKey, Message)> {
        let signed_message: Self = postcard::from_bytes(bytes)?;
        let key: PublicKey = signed_message.from;
        key.verify(&signed_message.data, &signed_message.signature)?;
        let message: Message = postcard::from_bytes(&signed_message.data)?;
        Ok((signed_message.from, message))
    }

    pub fn sign_and_encode(secret_key: &SecretKey, message: &Message) -> anyhow::Result<Vec<u8>> {
        let data = postcard::to_stdvec(&message)?;
        let signature = secret_key.sign(&data);
        let from: PublicKey = secret_key.public();
        let signed_message = Self {
            from,
            data,
            signature,
        };
        let encoded = postcard::to_stdvec(&signed_message)?;
        Ok(encoded)
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Message { text: String },
    // more message types will be added later
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

async fn handle_event(from: PublicKey, msg: Message) -> anyhow::Result<()> {
    match msg {
        Message::Message { text } => {
            println!("{}> {}", from, text);
        } // more message types will be added later
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
                    if let Event::Gossip(GossipEvent::Received(message)) = event {
                        let (from, msg) = SignedMessage::verify_and_decode(&message.content)?;
                        if let Err(cause) = handle_event(from, msg).await {
                            tracing::warn!("error handling message: {}", cause);
                        }
                    }
                } else {
                    break;
                }
            }
            line = stdin.next_line() => {
                if let Ok(Some(line)) = line {
                    let message = Message::Message { text: line.clone() };
                    let encoded = SignedMessage::sign_and_encode(&secret_key, &message)?;
                    gossip.broadcast(encoded.into()).await?;
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}
