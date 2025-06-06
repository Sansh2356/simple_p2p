// #![allow(unused)]
// use std::ops::Mul;

// use futures::{AsyncRead, AsyncWrite, StreamExt, executor::LocalPool};
// use libp2p::{
//     Multiaddr, PeerId, Swarm, SwarmBuilder, Transport,
//     core::{
//         muxing::StreamMuxerBox,
//         transport::{Boxed, MemoryTransport, OrTransport, upgrade},
//         upgrade::SelectUpgrade,
//     },
//     identify,
//     identity::{self, Keypair},
//     multiaddr::Protocol,
//     noise, ping, plaintext, quic, relay,
//     swarm::{Config, NetworkBehaviour, SwarmEvent},
//     tcp, yamux,
// };
// use relay::client;
// #[derive(NetworkBehaviour)]
// struct Behaviour {
//     relay: relay::client::Behaviour,
//     ping: ping::Behaviour,
//     identify: identify::Behaviour,
// }
// use clap::Parser;
// //manuall constructing the transport stack
// fn upgrade_transport<StreamSink>(
//     transport: Boxed<StreamSink>,
//     identity: &identity::Keypair,
// ) -> Boxed<(PeerId, StreamMuxerBox)>
// where
//     StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
// {
//     transport
//         .upgrade(upgrade::Version::V1)
//         .authenticate(noise::Config::new(&identity).unwrap())
//         .multiplex(yamux::Config::default())
//         .boxed()
// }
// async fn wait_for_reservation(
//     client: &mut Swarm<Behaviour>,
//     client_addr: Multiaddr,
//     relay_peer_id: PeerId,
//     is_renewal: bool,
// ) {
//     let mut new_listen_addr = false;
//     let mut reservation_req_accepted = false;
//     loop {
//         match client.select_next_some().await {
//             SwarmEvent::ExternalAddrConfirmed { address } if !is_renewal => {
//                 assert_eq!(address, client_addr);
//             }
//             SwarmEvent::Behaviour(BehaviourEvent::Relay(
//                 relay::client::Event::ReservationReqAccepted {
//                     relay_peer_id: peer_id,
//                     renewal,
//                     ..
//                 },
//             )) if relay_peer_id == peer_id && renewal == is_renewal => {
//                 println!(
//                     "RESERVATION HAS BEEN ACCEPTED from peer Id :{:?} and relay peerId {:?}",
//                     peer_id, relay_peer_id
//                 );
//                 reservation_req_accepted = true;
//                 if new_listen_addr {
//                     break;
//                 } else {
//                     break;
//                 }
//             }

//             SwarmEvent::NewListenAddr { address, .. } if address == client_addr => {
//                 new_listen_addr = true;
//                 if reservation_req_accepted {
//                     break;
//                 }
//             }
//             SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {}
//             e => println!(" {e:?}"),
//         }
//     }
// }
// async fn wait_for_dial(client: &mut Swarm<Behaviour>, remote: PeerId) -> bool {
//     loop {
//         match client.select_next_some().await {
//             SwarmEvent::Dialing {
//                 peer_id: Some(peer_id),
//                 ..
//             } if peer_id == remote => {}
//             SwarmEvent::ConnectionEstablished { peer_id, .. } if peer_id == remote => return true,
//             SwarmEvent::OutgoingConnectionError { peer_id, .. } if peer_id == Some(remote) => {
//                 return false;
//             }
//             SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {}
//             e => panic!("{e:?}"),
//         }
//     }
// }
// #[derive(Parser, Debug)]
// struct Args {
//     relay_peer_id: String,
//     relay_multiaddr: String,
// }
// #[tokio::main]
// async fn main() {
//     let args = Args::parse();
//     //generating the local key pair
//     let local_key = Keypair::generate_ed25519();
//     //generating the relay-client Transport and Network behaviour for relay-client
//     let (relay_transport, behaviour) = client::new(local_key.public().to_peer_id());
//     //Defining the network behaviour utlizing ping,client-relay and identify
//     let swarm_bheaviour = Behaviour {
//         ping: ping::Behaviour::new(ping::Config::new()),
//         relay: behaviour,
//         identify: identify::Behaviour::new(identify::Config::new(
//             String::from("/TODO/0.0.1"),
//             local_key.public(),
//         )),
//     };
//     //upgrading transport and mixing and building tcp/relay transport manually
//     let transport = upgrade_transport(
//         OrTransport::new(relay_transport, tcp::async_io::Transport::default()).boxed(),
//         &local_key,
//     );
//     //building the swarm manually
//     let mut swarm = Swarm::new(
//         transport,
//         swarm_bheaviour,
//         local_key.public().to_peer_id(),
//         Config::with_async_std_executor(),
//     );
//     let mut pool = LocalPool::new();
//     let relay_peer_id = args.relay_peer_id.parse::<PeerId>().unwrap();
//     let relay_multi_addr = args.relay_multiaddr.parse::<Multiaddr>().unwrap();
//     // println!()
//     // swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap());
//     //relay address
//     //dial address
//     let dial_addr = relay_multi_addr
//         .with(Protocol::P2p(relay_peer_id))
//         .with(Protocol::P2pCircuit)
//         .with(Protocol::P2p(local_key.public().to_peer_id()));
//     let addr_ref = dial_addr.clone();
//     println!("addr ref ---- : {:?}", addr_ref);
//     swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();
//     // assert!(pool.run_until(wait_for_dial(&mut swarm, relay_peer_id)));
//     // pool.run_until(wait_for_reservation(
//     //     &mut swarm,
//     //     addr_ref.clone(),
//     //     relay_peer_id,
//     //     false,
//     // ));
//     let a: Vec<&Multiaddr> = swarm.listeners().collect();
//     println!("{:?}  {:?}", a.len(),swarm.network_info());
//     for addr in swarm.listeners() {
//         println!("listening at - {:?}", addr);
//     }
//     loop {
//         match swarm.select_next_some().await {
//             SwarmEvent::NewListenAddr {
//                 listener_id,
//                 address,
//             } => {
//                 println!("{:?}",swarm.network_info());
//                 println!("Relay client listenening at {:?}", address);
//             }
//             //after establishing the connect of a peer behing a NAT/firewall
//             //checking whether the 1st phase of relay-v2 i.e. reservation via hop protocol is satisfied or not
//             SwarmEvent::Behaviour(BehaviourEvent::Relay(
//                 client::Event::ReservationReqAccepted {
//                     relay_peer_id,
//                     renewal,
//                     limit,
//                 },
//             )) => {
//                 println!("Intial reservation has been accepted by the relay node");
//             }
//             SwarmEvent::Dialing {
//                 peer_id,
//                 connection_id,
//             } => {
//                 println!("Dialing the relay node with peerId {:?}", peer_id);
//             }
//             SwarmEvent::ConnectionEstablished {
//                 peer_id,
//                 connection_id,
//                 endpoint,
//                 num_established,
//                 concurrent_dial_errors,
//                 established_in,
//             } => {
//                 println!(
//                     "Connection established with the relay successfully with the peer having peerid {:?}",
//                     peer_id
//                 );
//             }
//             SwarmEvent::OutgoingConnectionError {
//                 connection_id,
//                 peer_id,
//                 error,
//             } => {
//                 println!(
//                     "An error occurred while dialing the relay node ---- {:?}",
//                     error
//                 );
//             }
//             SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {
//                 println!("Ping event taking place");
//             }
//             SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Sent {
//                 connection_id,
//                 peer_id,
//             })) => {
//                 println!("Sent info to {:?}", peer_id);
//             }
//             SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
//                 connection_id,
//                 peer_id,
//                 info,
//             })) => {
//                 println!("Recieved info from and {:?}", info);
//             }
//             SwarmEvent::Behaviour(BehaviourEvent::Relay(
//                 relay::client::Event::InboundCircuitEstablished { src_peer_id, limit },
//             )) => {
//                 println!("INBOUND CIRCUIT ESTABLISHED");
//             }
//             SwarmEvent::Behaviour(BehaviourEvent::Relay(
//                 relay::client::Event::OutboundCircuitEstablished {
//                     relay_peer_id,
//                     limit,
//                 },
//             )) => {
//                 println!("OUTBOUND CIRCUIT ESTABLISHED");
//             }
//             _ => {}
//         }
//     }
// }

#![allow(unused)]
use std::{ops::Mul, time::Duration};

use futures::{AsyncRead, AsyncWrite, StreamExt, executor::LocalPool};
use libp2p::{
    Multiaddr, PeerId, Swarm, SwarmBuilder, Transport,
    core::{
        muxing::StreamMuxerBox,
        transport::{Boxed, MemoryTransport, OrTransport, upgrade},
        upgrade::SelectUpgrade,
    },
    identify,
    identity::{self, Keypair},
    multiaddr::Protocol,
    noise, ping, plaintext, quic, relay,
    swarm::{Config, NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use relay::client;
#[derive(NetworkBehaviour)]
struct Behaviour {
    relay: relay::client::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}
use clap::Parser;
//manuall constructing the transport stack
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
async fn wait_for_reservation(
    client: &mut Swarm<Behaviour>,
    client_addr: Multiaddr,
    relay_peer_id: PeerId,
    is_renewal: bool,
) {
    let mut new_listen_addr = false;
    let mut reservation_req_accepted = false;
    loop {
        match client.select_next_some().await {
            SwarmEvent::ExternalAddrConfirmed { address } if !is_renewal => {
                assert_eq!(address, client_addr);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(
                relay::client::Event::ReservationReqAccepted {
                    relay_peer_id: peer_id,
                    renewal,
                    ..
                },
            )) if relay_peer_id == peer_id && renewal == is_renewal => {
                println!(
                    "RESERVATION HAS BEEN ACCEPTED from peer Id :{:?} and relay peerId {:?}",
                    peer_id, relay_peer_id
                );
                reservation_req_accepted = true;
                if new_listen_addr {
                    break;
                } else {
                    break;
                }
            }

            SwarmEvent::NewListenAddr { address, .. } if address == client_addr => {
                new_listen_addr = true;
                if reservation_req_accepted {
                    break;
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {}
            e => println!(" {e:?}"),
        }
    }
}
async fn wait_for_dial(client: &mut Swarm<Behaviour>, remote: PeerId) -> bool {
    loop {
        match client.select_next_some().await {
            SwarmEvent::Dialing {
                peer_id: Some(peer_id),
                ..
            } if peer_id == remote => {}
            SwarmEvent::ConnectionEstablished { peer_id, .. } if peer_id == remote => return true,
            SwarmEvent::OutgoingConnectionError { peer_id, .. } if peer_id == Some(remote) => {
                return false;
            }
            SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {}
            e => panic!("{e:?}"),
        }
    }
}
#[derive(Parser, Debug)]
struct Args {
    relay_peer_id: String,
    relay_multiaddr: String,
}
#[tokio::main]
async fn main() {
    let args = Args::parse();
    //generating the local key pair
    let local_key = Keypair::generate_ed25519();
    //generating the relay-client Transport and Network behaviour for relay-client
    let (relay_transport, behaviour) = client::new(local_key.public().to_peer_id());
    //Defining the network behaviour utlizing ping,client-relay and identify
    let swarm_bheaviour = Behaviour {
        ping: ping::Behaviour::new(ping::Config::new().with_timeout(Duration::from_secs(3600))),
        relay: behaviour,
        identify: identify::Behaviour::new(identify::Config::new(
            String::from("/TODO/0.0.1"),
            local_key.public(),
        )),
    };
    //upgrading transport and mixing and building tcp/relay transport manually
    let transport = upgrade_transport(
        OrTransport::new(relay_transport, tcp::async_io::Transport::default()).boxed(),
        &local_key,
    );
    //building the swarm manually
    let mut swarm = Swarm::new(
        transport,
        swarm_bheaviour,
        local_key.public().to_peer_id(),
        Config::with_async_std_executor().with_idle_connection_timeout(Duration::from_secs(3600)),
    );
    let mut pool = LocalPool::new();
    let relay_peer_id = args.relay_peer_id.parse::<PeerId>().unwrap();
    let relay_multi_addr = args.relay_multiaddr.parse::<Multiaddr>().unwrap();
    // swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap());
    //relay address
    //dial address
    let dial_addr = relay_multi_addr
        .with(Protocol::P2p(relay_peer_id))
        .with(Protocol::P2pCircuit)
        .with(Protocol::P2p(local_key.public().to_peer_id()));
    let addr_ref = dial_addr.clone();
    println!("addr ref ---- : {:?}", addr_ref);
    swarm.listen_on(dial_addr).unwrap();
    assert!(pool.run_until(wait_for_dial(&mut swarm, relay_peer_id)));
    pool.run_until(wait_for_reservation(
        &mut swarm,
        addr_ref.clone(),
        relay_peer_id,
        false,
    ));
    println!("CURRENT SWARM LOCAL PEERID {:?}",swarm.local_peer_id());
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
                println!("Dialing the node with peerId {:?}", peer_id);
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
                    "Connection established with the relay successfully with the peer having peerid {:?} and address {:?}",
                    peer_id,
                    endpoint.get_remote_address()
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
            SwarmEvent::Behaviour(BehaviourEvent::Ping(ping_event)) => {
                // ping_event.
                println!(
                    "Ping event taking place from a peer having peer id and result {:?} {:?}",
                    ping_event.peer, ping_event.result
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
            SwarmEvent::Behaviour(BehaviourEvent::Relay(
                relay::client::Event::InboundCircuitEstablished { src_peer_id, limit },
            )) => {
                println!("INBOUND CIRCUIT ESTABLISHED");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(
                relay::client::Event::OutboundCircuitEstablished {
                    relay_peer_id,
                    limit,
                },
            )) => {
                println!("OUTBOUND CIRCUIT ESTABLISHED");
            }
            _ => {}
        }
    }
}
