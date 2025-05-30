// use std::{error::Error, time::Duration};

// use futures::prelude::*;
// use libp2p::{noise, ping::{self, Event}, swarm::SwarmEvent, tcp, yamux, Multiaddr,quic};
// use tracing_subscriber::EnvFilter;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     let _ = tracing_subscriber::fmt()
//         .with_env_filter(EnvFilter::from_default_env())
//         .try_init();

//     let mut swarm = libp2p::SwarmBuilder::with_new_identity()
//         .with_tokio()
//         .with_tcp(
//             tcp::Config::default(),
//             noise::Config::new,
//             yamux::Config::default,
//         )?
//         .with_behaviour(|_| ping::Behaviour::default())?
//         .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX)))
//         .build();

//     // Tell the swarm to listen on all interfaces and a random, OS-assigned
//     // port.
//     swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

//     // Dial the peer identified by the multi-address given as the second
//     // command-line argument, if any.
//     if let Some(addr) = std::env::args().nth(1) {
//         let remote: Multiaddr = addr.parse()?;
//         swarm.dial(remote)?;
//         println!("Dialed {addr}")
//     }
//     loop {
//         match swarm.select_next_some().await {
//             SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
//             SwarmEvent::Behaviour(event) => println!("{event:?}"),
//             _ => {}
//         }
//     }
// }

// use criterion::{Criterion, criterion_group, criterion_main};
use futures::prelude::*;
use libp2p::{
    Multiaddr,
    ping::{self, Behaviour, Event},
    swarm::{Swarm, SwarmEvent},
};
use std::time::Duration;
use tokio::runtime::Runtime;

fn build_swarm_quic() -> Swarm<Behaviour> {
    libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_quic()
        .with_behaviour(|_| ping::Behaviour::default())
        .unwrap()
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(30))) // Reduced from MAX for practicality
        .build()
}

fn build_swarm_tcp() -> Swarm<Behaviour> {
    libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .unwrap()
        .with_behaviour(|_| ping::Behaviour::default())
        .unwrap()
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(30)))
        .build()
}

async fn run_ping(mut swarm1: Swarm<Behaviour>, mut swarm2: Swarm<Behaviour>, proto: &str) {
    let listen_addr: Multiaddr = match proto {
        "quic" => "/ip4/127.0.0.1/udp/0/quic-v1".parse().unwrap(),
        "tcp" => "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        _ => panic!("Unknown protocol"),
    };

    swarm1.listen_on(listen_addr).expect("Failed to listen");

    let mut addr = None;
    while let Some(event) = swarm1.next().await {
        if let SwarmEvent::NewListenAddr { address, .. } = event {
            addr = Some(address);
            break;
        }
    }

    let remote = addr.expect("Failed to get listen address");
    swarm2.dial(remote).expect("Failed to dial");

    let mut ping_count = 0;
    let mut events = futures::stream::select(swarm1, swarm2);

    while let Some(event) = events.next().await {
        if let SwarmEvent::Behaviour(event) = event {
            println!("EVENT === {:?}", event);
            println!("PING COUNT --- {:?}",ping_count);
            ping_count += 1;
            if ping_count >= 10 {
                break;
            }
        }
    }
}
#[tokio::main]
async fn main() {
    let s1 = build_swarm_tcp();
    let s2 = build_swarm_tcp();
    run_ping(s1, s2, "tcp").await;
}
