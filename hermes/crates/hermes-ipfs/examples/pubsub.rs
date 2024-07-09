//! `PubSub` example
//!
//! This example starts two bootstrapped Hermes IPFS Nodes and subscribes them to the
//! `ipfs-chat` topic, creating an async stream for incoming topic publications for each
//! node. Another set of async streams are created for swarm pubsub events related to the
//! topic.
//!
//! The example then spawns different async tasks that run in a loop.
//!
//! * The task that reads from the topic stream for Node A
//! * The task that reads from the topic stream for Node B
//! * The task that reads from the topic swarm events for Node A
//! * The task that reads from the topic swarm events for Node B
//! * The task that reads lines from stdin and publishes them as either node.
use std::io::Write;

use hermes_ipfs::{pin_mut, FutureExt, HermesIpfs, StreamExt};
use rust_ipfs::PubsubEvent;
use rustyline_async::Readline;

#[allow(clippy::indexing_slicing)]
/// Connect Node A, upload file and provide CID by adding to DHT
async fn start_bootstrapped_nodes() -> anyhow::Result<(HermesIpfs, HermesIpfs)> {
    let hermes_a = HermesIpfs::start().await?;
    println!("***************************************");
    println!("* Hermes IPFS node A has started.");
    let peer_id_a = hermes_a.identity(None).await?;
    println!("    Peer ID: {peer_id_a}");
    let addresses = hermes_a.listening_addresses().await?;
    let a_address = addresses[0].clone();
    let a_p2p = a_address.with(rust_ipfs::Protocol::P2p(peer_id_a));
    println!("    P2P addr: {a_p2p}");
    println!("***************************************");
    println!("* Hermes IPFS node B has started.");
    let hermes_b = HermesIpfs::start().await?;
    let peer_id_b = hermes_b.identity(None).await?;
    println!("    Peer ID: {peer_id_b}");
    let addresses = hermes_b.listening_addresses().await?;
    let b_address = addresses[0].clone();
    let b_p2p = b_address.with(rust_ipfs::Protocol::P2p(peer_id_b));
    println!("    P2P addr: {b_p2p}");
    println!("***************************************");
    println!("* Bootstrapping node A.");
    hermes_a.dht_mode(rust_ipfs::DhtMode::Server).await?;
    hermes_a.add_bootstrap(b_p2p).await?;
    hermes_a.bootstrap().await?;
    println!("***************************************");
    println!("* Bootstrapping node B.");
    hermes_b.dht_mode(rust_ipfs::DhtMode::Server).await?;
    hermes_b.add_bootstrap(a_p2p).await?;
    hermes_b.bootstrap().await?;
    println!("***************************************");
    Ok((hermes_a, hermes_b))
}

#[tokio::main]
/// Main function
async fn main() -> anyhow::Result<()> {
    let topic = String::from("ipfs-chat");

    // Initialize the repo and start a daemon
    let (hermes_a, hermes_b) = start_bootstrapped_nodes().await?;
    let (mut rl, mut stdout) = Readline::new(format!("{} > ", "Write message to publish"))?;

    let mut event_stream = hermes_a.pubsub_events(&topic).await?;
    let mut event_stream_b = hermes_b.pubsub_events(&topic).await?;

    let stream = hermes_a.pubsub_subscribe(topic.to_string()).await?;
    let stream_b = hermes_b.pubsub_subscribe(topic.to_string()).await?;

    pin_mut!(stream);
    pin_mut!(stream_b);

    tokio::task::yield_now().await;

    let mut peer_line = PeerLine::A;
    loop {
        tokio::select! {
            data = stream.next() => {
                if let Some(msg) = data {
                    writeln!(stdout, "NODE A RECV: {}", String::from_utf8_lossy(&msg.data))?;
                }
            }
            data = stream_b.next() => {
                if let Some(msg) = data {
                    writeln!(stdout, "NODE B RECV: {}", String::from_utf8_lossy(&msg.data))?;
                }
            }
            Some(event) = event_stream.next() => {
                match event {
                    PubsubEvent::Subscribe { peer_id } => writeln!(stdout, "{peer_id} subscribed")?,
                    PubsubEvent::Unsubscribe { peer_id } => writeln!(stdout, "{peer_id} unsubscribed")?,
                }
            }
            Some(event) = event_stream_b.next() => {
                match event {
                    PubsubEvent::Subscribe { peer_id } => writeln!(stdout, "{peer_id} subscribed")?,
                    PubsubEvent::Unsubscribe { peer_id } => writeln!(stdout, "{peer_id} unsubscribed")?,
                }
            }
            line = rl.readline().fuse() => match line {
                Ok(rustyline_async::ReadlineEvent::Line(line)) => {
                    let line_bytes = line.as_bytes().to_vec();
                    let topic = topic.clone();
                    match peer_line {
                        PeerLine::A => {
                            if let Err(e) = hermes_a.pubsub_publish(topic, line_bytes).await {
                                writeln!(stdout, "Error publishing message: {e}")?;
                                continue;
                            }
                        }
                        PeerLine::B => {
                            if let Err(e) = hermes_b.pubsub_publish(topic, line_bytes).await {
                                writeln!(stdout, "Error publishing message: {e}")?;
                                continue;
                            }
                        }
                    }
                    writeln!(stdout, "{peer_line} SEND: {line}")?;
                    peer_line.toggle();
                }
                Ok(rustyline_async::ReadlineEvent::Eof | rustyline_async::ReadlineEvent::Interrupted) => {
                    break
                },
                Err(e) => {
                    writeln!(stdout, "Error: {e}")?;
                    writeln!(stdout, "Exiting...")?;
                    break
                },
            }
        }
    }
    // Exit
    hermes_a.stop().await;
    Ok(())
}

#[derive(Debug)]
/// Helpful enum for toggling which peer reads lines from stdin
enum PeerLine {
    /// Node A
    A,
    /// Node B
    B,
}

impl PeerLine {
    /// Toggle peers
    fn toggle(&mut self) {
        *self = match self {
            PeerLine::A => PeerLine::B,
            PeerLine::B => PeerLine::A,
        };
    }
}

impl std::fmt::Display for PeerLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NODE {self:?}")
    }
}
