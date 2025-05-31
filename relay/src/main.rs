#![allow(unused)]
use std::{
    error::Error,
    net::{Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use clap::Parser;
use futures::{AsyncRead, AsyncWrite, StreamExt};
use libp2p::{
    PeerId, Swarm, Transport,
    core::{
        Multiaddr,
        multiaddr::Protocol,
        muxing::StreamMuxerBox,
        transport::{Boxed, upgrade},
    },
    identify, identity, noise, ping, relay,
    swarm::{Config, NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use tracing_subscriber::EnvFilter;
//manually constructing the transport stack
fn upgrade_transport<StreamSink>(
    transport: Boxed<StreamSink>,
    identity: &identity::Keypair,
) -> Boxed<(PeerId, StreamMuxerBox)>
where
    StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    transport
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::Config::new(&identity).unwrap())
        .multiplex(yamux::Config::default())
        .boxed()
}
#[tokio::main]

async fn main() -> Result<(), Box<dyn Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let local_key: identity::Keypair = identity::Keypair::generate_ed25519();
    let transport = upgrade_transport(tcp::async_io::Transport::default().boxed(), &local_key);
    let net_behaviour = Behaviour {
        identify: identify::Behaviour::new(identify::Config::new(
            "/TODO/0.0.1".to_string(),
            local_key.public(),
        )),
        relay: relay::Behaviour::new(
            local_key.public().to_peer_id(),
            relay::Config {
                reservation_duration: Duration::from_secs(2),
                ..Default::default()
            },
        ),
        ping: ping::Behaviour::new(ping::Config::new()),
    };
    let mut swarm = Swarm::new(
        transport,
        net_behaviour,
        local_key.public().to_peer_id(),
        Config::with_async_std_executor(),
    );
    let relay_local_peer_id = swarm.local_peer_id();
    let relay_local_ref = relay_local_peer_id.clone();
    let relay_addr = "/ip4/0.0.0.0/tcp/0";
    swarm.listen_on(relay_addr.clone().parse()?).unwrap();
    swarm.add_external_address(relay_addr.clone().parse()?);
    println!("Relay server local peerID is {:?}", relay_local_ref);
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(BehaviourEvent::Relay(relay::Event::CircuitClosed {
                src_peer_id,
                dst_peer_id,
                error,
            })) => {
                println!(
                    "AN INBOUND CIRCUIT REQUEST BETWEEN SOURCE AND DESTINATION HAS BEEN CLOSED {:?} and {:?} due to {:?}",
                    src_peer_id, dst_peer_id, error
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(relay::Event::CircuitReqAccepted {
                src_peer_id,
                dst_peer_id,
            })) => {
                println!(
                    "AN INBOUND CIRCUIT REQUEST BETWEEN SOURCE AND DESTINATION HAS BEEN ACCEPTED  {:?} and {:?}",
                    src_peer_id, dst_peer_id
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(relay::Event::CircuitReqDenied {
                src_peer_id,
                dst_peer_id,
            })) => {
                println!(
                    "AN INBOUND CIRCUIT REQUEST BETWEEN SOURCE AND DESTINATION HAS BEEN DENIED {:?} and {:?}",
                    src_peer_id, dst_peer_id
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(relay::Event::ReservationReqDenied {
                src_peer_id,
            })) => {
                println!(
                    "Intial reservation has been denied by the relay node from the src_peer_id {:?}",
                    src_peer_id
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(relay::Event::CircuitReqAccepted {
                src_peer_id,
                dst_peer_id,
            })) => {}
            SwarmEvent::Behaviour(BehaviourEvent::Relay(
                relay::Event::ReservationReqAccepted {
                    src_peer_id,
                    renewed,
                },
            )) => {
                println!(
                    "Intial reservation has been accepted by the relay node from the src_peer_id {:?}",
                    src_peer_id
                );
            }
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                println!(
                    "relay server listening on address {:?}",
                    address.to_string()
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {
                println!("Received ping from some peer");
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
                established_in,
            } => {
                println!(
                    "Connection established with a new peer with peer id {:?}",
                    peer_id
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Sent {
                connection_id,
                peer_id,
            })) => {
                println!("Sent info to {:?}", peer_id);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                connection_id,
                peer_id,
                info,
            })) => {
                println!("Recieved info from and {:?}", info);
            }
            _ => {}
        }
    }
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    relay: relay::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

fn generate_ed25519(secret_key_seed: u8) -> identity::Keypair {
    let mut bytes = [0u8; 32];
    bytes[0] = secret_key_seed;

    identity::Keypair::ed25519_from_bytes(bytes).expect("only errors on wrong length")
}

// let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
//     .with_tokio()
//     .with_tcp(
//         tcp::Config::default(),
//         noise::Config::new,
//         yamux::Config::default,
//     )?
//     .with_quic()
//     .with_behaviour(|key| Behaviour {
//         identify: identify::Behaviour::new(identify::Config::new(
//             "/TODO/0.0.1".to_string(),
//             key.public(),
//         )),
//         relay: relay::Behaviour::new(
//             key.public().to_peer_id(),
//             relay::Config {
//                 reservation_duration: Duration::from_secs(2),
//                 ..Default::default()
//             },
//         ),
//         ping: ping::Behaviour::new(ping::Config::new()),
//     })?
//     .build();
