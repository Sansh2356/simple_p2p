//! Publish/subscribe chat over [`iroh-gossip`].
//!
//! Every peer that subscribes to the same topic forms a gossip swarm and
//! broadcasts to all the others. Connectivity is dial-by-key (n0 discovery), so
//! a joining peer only needs the **Endpoint ID** of one peer already in the
//! swarm to bootstrap — no IPs, no relay configuration.
//!
//! First peer (opens the room, has no one to bootstrap from):
//! ```bash
//! cargo run -- --room demo
//! ```
//! It prints its Endpoint ID. Other peers join by passing it:
//! ```bash
//! cargo run -- --room demo <endpoint-id-of-first-peer>
//! ```
//! Then type lines into stdin; every peer in the room sees them.

use anyhow::{Context, Result};
use clap::Parser;
use futures_lite::StreamExt;
use iroh::{Endpoint, EndpointId, endpoint::presets};
use iroh_gossip::{
    api::Event,
    net::Gossip,
    proto::TopicId,
};
use sha2::{Digest, Sha256};

#[derive(Parser)]
struct Args {
    /// Room name — hashed into the gossip topic id. All peers must match.
    #[arg(long, default_value = "demo")]
    room: String,
    /// Endpoint IDs of peers already in the room to bootstrap from.
    /// Leave empty for the first peer.
    bootstrap: Vec<EndpointId>,
}

/// Derive a 32-byte [`TopicId`] from a human-readable room name.
fn topic_for(room: &str) -> TopicId {
    let digest = Sha256::digest(room.as_bytes());
    TopicId::from_bytes(digest.into())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let topic = topic_for(&args.room);

    let endpoint = Endpoint::builder(presets::N0)
        .alpns(vec![iroh_gossip::ALPN.to_vec()])
        .bind()
        .await?;
    println!("our endpoint id: {}", endpoint.id());
    println!("room {:?} -> topic {}", args.room, topic);

    // Spawn the gossip protocol on this endpoint and wire it into a router so
    // it answers incoming gossip connections.
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let _router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    if args.bootstrap.is_empty() {
        println!("first peer in the room — share the endpoint id above so others can join.");
    } else {
        println!("bootstrapping from {} peer(s)...", args.bootstrap.len());
    }

    // Join the swarm. With bootstrap peers this waits until we are connected to
    // at least one of them; for the first peer it returns immediately.
    let (sender, mut receiver) = gossip
        .subscribe_and_join(topic, args.bootstrap)
        .await
        .context("failed to join gossip topic")?
        .split();
    println!("joined the room — type a message and press enter:");

    // Read stdin lines and broadcast each one to the swarm.
    let send_task = tokio::spawn(async move {
        let mut lines = tokio::io::AsyncBufReadExt::lines(tokio::io::BufReader::new(
            tokio::io::stdin(),
        ));
        while let Ok(Some(line)) = lines.next_line().await {
            if line.is_empty() {
                continue;
            }
            if let Err(err) = sender.broadcast(line.into_bytes().into()).await {
                eprintln!("broadcast error: {err:#}");
                break;
            }
        }
    });

    // Print events from the swarm: messages and membership changes.
    while let Some(event) = receiver.try_next().await? {
        match event {
            Event::Received(msg) => {
                let from = msg.delivered_from;
                let text = String::from_utf8_lossy(&msg.content);
                println!("[{}] {text}", from.fmt_short());
            }
            Event::NeighborUp(id) => println!("* {} joined", id.fmt_short()),
            Event::NeighborDown(id) => println!("* {} left", id.fmt_short()),
            _ => {}
        }
    }

    send_task.abort();
    Ok(())
}
