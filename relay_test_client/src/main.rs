#![allow(unused)]
use std::ops::Mul;

use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, identify,
    identity::Keypair,
    multiaddr::Protocol,
    noise, ping, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use relay::client;
#[derive(NetworkBehaviour)]
struct Behaviour {
    relay: relay::client::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}
#[tokio::main]
async fn main() {
    let local_key = Keypair::generate_ed25519();
    let (relay_transport, behaviour) = client::new(local_key.public().to_peer_id());
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_behaviour(|key| Behaviour {
            ping: ping::Behaviour::new(ping::Config::new()),
            relay: behaviour,
            identify: identify::Behaviour::new(identify::Config::new(
                String::from("/TODO/0.0.1"),
                key.public(),
            )),
        })
        .unwrap()
        .build();
    let relay_addr: Multiaddr = "/ip4/127.0.0.1/tcp/44387".parse().unwrap();
    let relay_peer_id: PeerId = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN"
        .parse()
        .unwrap();
    let client_addr = relay_addr.clone().with(Protocol::P2p(relay_peer_id));
    // .with(Protocol::P2pCircuit);
    let client_peer_id = swarm.local_peer_id();
let remote_peer_id_str = client_peer_id.to_string();
// let formatted_str = format!();
    swarm.listen_on("/ip4/74.50.123.158/tcp/8888/p2p-circuit".parse().unwrap());
    // let test_addr_1:Multiaddr = "/ip4/127.0.0.1/tcp/44387/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN".parse().unwrap();
    let test_addr_1: Multiaddr =
        "/ip4/74.50.123.158/tcp/8888/p2p-circuit/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN"
            .parse()
            .unwrap();
    //dialing the realy node to which the connection is to be established
    swarm.dial(test_addr_1).unwrap();
    let mut reservation_req_accepted = false;

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                println!("Relay client listenening at {:?}", address);
            }
            //after establishing the connect of a peer behing a NAT/firewall
            //checking whether the 1st phase of relay-v2 i.e. reservation via hop protocol is satisfied or not
            SwarmEvent::Behaviour(BehaviourEvent::Relay(
                client::Event::ReservationReqAccepted {
                    relay_peer_id,
                    renewal,
                    limit,
                },
            )) => {
                println!("Intial reservation has been accepted by the relay node");
            }
            SwarmEvent::Dialing {
                peer_id,
                connection_id,
            } => {
                println!("Dialing the relay node with peerId {:?}", peer_id);
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
                    "Connection established with the relay successfully with the relay peerid {:?}",
                    peer_id
                );
            }
            SwarmEvent::OutgoingConnectionError {
                connection_id,
                peer_id,
                error,
            } => {
                println!(
                    "An error occurred while dialing the relay node ---- {:?}",
                    error
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {
                println!("Ping event taking place");
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
