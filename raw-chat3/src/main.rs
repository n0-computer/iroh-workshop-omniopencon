use std::str::FromStr;

use clap::Parser;
use futures::StreamExt;
use iroh_base::node_addr::AddrInfoOptions;
use iroh_gossip::{
    net::{Event, Gossip, GossipEvent, GossipTopic},
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
    Direct { to: PublicKey, encrypted: Vec<u8> },
    // more message types will be added later
}

/// Handle incoming connections by dispatching them to the right handler.
async fn handle_connections(endpoint: Endpoint, gossip: Gossip) -> anyhow::Result<()> {
    while let Some(mut connecting) = endpoint.accept().await {
        let gossip = gossip.clone();
        tokio::spawn(async move {
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

async fn handle_event(from: PublicKey, secret_key: SecretKey, msg: Message) -> anyhow::Result<()> {
    match msg {
        Message::Message { text } => {
            println!("{}> {}", from, text);
        }
        Message::Direct { to, encrypted } => {
            if to != secret_key.public() {
                // not for us
                return Ok(());
            }
            let mut buffer = encrypted;
            secret_key.shared(&from).open(&mut buffer)?;
            let message = std::str::from_utf8(&buffer)?;
            println!("got encrypted message from {}: {}", from, message);
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
        .bind(0)
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
                        if let Err(cause) = handle_event(from, secret_key.clone(), msg).await {
                            tracing::warn!("error handling message: {}", cause);
                        }
                    }
                } else {
                    break;
                }
            }
            line = stdin.next_line() => {
                if let Ok(Some(line)) = line {
                    if let Err(cause) = send_message(&mut gossip, line, &secret_key).await {
                        tracing::warn!("error sending message: {}", cause);
                    }
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}

async fn send_message(
    gossip: &mut GossipTopic,
    line: String,
    secret_key: &SecretKey,
) -> anyhow::Result<()> {
    let msg = if let Some(private) = line.strip_prefix("/for ") {
        // yeah yeah, there are nicer ways to do this, sue me...
        let mut parts = private.splitn(2, ' ');
        let Some(to) = parts.next() else {
            anyhow::bail!("missing recipient");
        };
        let Some(msg) = parts.next() else {
            anyhow::bail!("missing message");
        };
        let Ok(to) = PublicKey::from_str(to) else {
            anyhow::bail!("invalid recipient");
        };
        let mut encrypted = msg.as_bytes().to_vec();
        // encrypt the data in place
        secret_key.shared(&to).seal(&mut encrypted);
        Message::Direct { to, encrypted }
    } else {
        Message::Message { text: line }
    };
    let msg = SignedMessage::sign_and_encode(secret_key, &msg)?;
    gossip.broadcast(msg.into()).await?;
    Ok(())
}
