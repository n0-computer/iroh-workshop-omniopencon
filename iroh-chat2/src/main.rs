use clap::Parser;
use futures::{SinkExt, StreamExt};
use iroh::{
    base::node_addr::AddrInfoOptions,
    gossip::net::{Command, Event, GossipEvent},
    net::{
        key::{PublicKey, SecretKey, Signature},
        ticket::NodeTicket,
    },
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncBufReadExt, select};
use util::wait_for_relay;
mod util;

#[derive(Debug, Parser)]
struct Args {
    tickets: Vec<NodeTicket>,
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Message { text: String },
    // more message types will be added later
}

#[derive(Debug, Serialize, Deserialize)]
struct SignedMessage {
    from: PublicKey,
    data: Vec<u8>,
    signature: Signature,
    uid: u128,
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
        let uid = rand::thread_rng().gen();
        let signed_message = Self {
            from,
            data,
            signature,
            uid,
        };
        let encoded = postcard::to_stdvec(&signed_message)?;
        Ok(encoded)
    }
}

async fn handle_event(event: Event) {
    match event {
        Event::Gossip(ev) => match ev {
            GossipEvent::Received(msg) => {
                let Ok((from, msg)) = SignedMessage::verify_and_decode(&msg.content) else {
                    tracing::warn!(
                        "Failed to verify message from node {}: {:?}",
                        msg.delivered_from,
                        msg.content
                    );
                    return;
                };
                match msg {
                    Message::Message { text } => {
                        println!("Received message from node {}: {}", from, text);
                    }
                }
            }
            other => {
                tracing::info!("Got other event: {:?}", other);
            }
        },
        Event::Lagged => {
            tracing::info!("Missed some messages");
        }
    }
}

async fn parse_as_command(line: String, secret_key: &SecretKey) -> anyhow::Result<Option<Command>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let msg = Message::Message {
        text: trimmed.to_string(),
    };
    let signed = SignedMessage::sign_and_encode(secret_key, &msg)?;
    let cmd = Command::Broadcast(signed.into());
    Ok(Some(cmd))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // log to console, using the RUST_LOG environment variable
    tracing_subscriber::fmt::init();
    // parse command line arguments
    let args = Args::parse();
    // get or create the secret key / node identity
    let secret_key = util::get_or_create_secret()?;
    // create a new Iroh node, giving it the secret key
    let iroh = iroh::node::Node::memory()
        .secret_key(secret_key.clone())
        .spawn()
        .await?;
    // wait for the node to figure out its own home relay
    wait_for_relay(iroh.endpoint()).await?;
    // print node addr and ticket, both long and short
    let mut my_addr = iroh.endpoint().node_addr().await?;
    let ticket = NodeTicket::new(my_addr.clone())?;
    println!("I am {}", my_addr.node_id);
    println!("Connect to me using cargo run {}", ticket);
    my_addr.apply_options(AddrInfoOptions::Id);
    let short = NodeTicket::new(my_addr.clone())?;
    println!("..or using          cargo run {}", short);
    // add all the info from the tickets to the endpoint
    // also extract the node IDs to use as bootstrap nodes
    let mut bootstrap = Vec::new();
    for ticket in &args.tickets {
        let addr = ticket.node_addr();
        iroh.endpoint().add_node_addr(addr.clone()).ok();
        bootstrap.push(addr.node_id);
    }
    // hardcoded topic
    let topic = [0u8; 32];
    // subscribe to the topic, giving the bootstrap nodes
    // if the tickets contained additional info, this is available in the address book of the endpoint
    let (mut sink, mut stream) = iroh.gossip().subscribe(topic, bootstrap).await?;
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    loop {
        select! {
            message = stream.next() => {
                // got a message from the gossip network
                if let Some(Ok(event)) = message {
                    handle_event(event).await;
                } else {
                    break;
                }
            }
            line = stdin.next_line() => {
                if let Ok(Some(line)) = line {
                    // got a line from stdin
                    match parse_as_command(line, &secret_key).await {
                        Ok(cmd) => {
                            if let Some(cmd) = cmd {
                                sink.send(cmd).await?;
                            }
                        }
                        Err(cause) => {
                            tracing::warn!("error parsing command: {}", cause);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
