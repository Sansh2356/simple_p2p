// use std::{error::Error, time::Duration};

// use futures::StreamExt;
// use libp2p::{
//     core::multiaddr::Multiaddr,
//     identify, noise, ping,
//     swarm::{NetworkBehaviour, SwarmEvent},
//     tcp, yamux,
// };
// use tracing_subscriber::EnvFilter;
// #[derive(NetworkBehaviour)]
// struct MyBehaviour {
//     ping: ping::Behaviour,
//     identify: identify::Behaviour,
// }
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
//         .with_behaviour(|key| MyBehaviour {
//             ping: ping::Behaviour::new(ping::Config::default()),
//             identify: identify::Behaviour::new(identify::Config::new("".to_string(), key.public())),
//         })?
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
//             SwarmEvent::Behaviour(MyBehaviourEvent::Ping(event)) => {
//                 println!(
//                     "RECIEVED PING FROM A PEER  {:?} , {:?} , {:?}",
//                     event.connection, event.peer, event.result
//                 );
//             }
//             SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
//             // Prints peer id identify info is being sent to.
//             SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Sent {
//                 peer_id,
//                 ..
//             })) => {
//                 println!("Sent identify info to {peer_id:?}")
//             }
//             // Prints out the info received via the identify event
//             SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Received {
//                 info,
//                 ..
//             })) => {
//                 println!("Received {info:?}")
//             }
//             event => {
//                 println!("{:?}",event);
//             }
//         }
//     }
// }

#![allow(unused)]

use std::{
    collections::hash_map::DefaultHasher,
    env,
    error::Error,
    hash::{Hash, Hasher},
    time::Duration,
};

use futures::stream::StreamExt;
use libp2p::{
    Multiaddr, StreamProtocol,
    core::transport::ListenerId,
    kad::{Mode, QueryResult, store::MemoryStore},
};
use libp2p::{
    PeerId, dns, gossipsub, identify, identity, kad, mdns, noise, ping,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use std::fs;
use std::path::Path;
use tokio::{io, io::AsyncBufReadExt, select};
use tracing_subscriber::EnvFilter;
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    kad: kad::Behaviour<MemoryStore>,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let path = Path::new("/home/sansh2356/simple_p2p/example_2/src/key_pair.gpg");
    let fetched_value =
        fs::read(&path).expect("Error occurred while reading key bytes from the .gpg file");

    let key_pair = identity::Keypair::from_protobuf_encoding(&fetched_value)
        .expect("An error occurred while decoding a keypair from protobuf encoding");

    const KADPROTOCOLNAME: StreamProtocol = StreamProtocol::new("/braidpool/kad/1.0.0");
    const IDENTIFYPROTOCOLNAME: StreamProtocol = StreamProtocol::new("/braidpool/identify/1.0.0");
    let dns_link = "/dns4/french.braidpool.net/udp/8888/quic-v1";
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
    println!("{:?}", key_pair.public().to_peer_id());
    let mut swarm: libp2p::Swarm<MyBehaviour> =
        libp2p::SwarmBuilder::with_existing_identity(key_pair)
            .with_tokio()
            .with_quic()
            .with_dns()
            .unwrap()
            .with_behaviour(|key| {
                println!("{:?}", key.public().to_peer_id());

                //initializing the store for kademlia based DHT
                let store = MemoryStore::new(key.public().to_peer_id());
                //custom kademlia protocol
                let mut kad_config = kad::Config::new(StreamProtocol::new("/braidpool/kad/1.0.0"));
                kad_config.set_query_timeout(tokio::time::Duration::from_secs(60));
                //custom kad configuration
                let kademlia_behaviour =
                    kad::Behaviour::with_config(key.public().to_peer_id(), store, kad_config);
                let protocol_names = kademlia_behaviour.protocol_names();
                for protocol_name in protocol_names {
                    println!("protocol names are - : {:?}", protocol_name.to_string());
                }

                //identify protocol configuration
                let identify_config =
                    identify::Config::new("/braidpool/identify/1.0.0".to_string(), key.public());
                let identify = identify::Behaviour::new(identify_config);
                //custom network behaviour stack from libp2p
                let ping_config = ping::Config::default();
                let ping_behaviour = ping::Behaviour::new(ping_config);
                Ok(MyBehaviour {
                    kad: kademlia_behaviour,
                    identify,
                    ping: ping_behaviour,
                })
            })?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
            })
            .build();

    let mut args = env::args().skip(1);
    if let (Some(peer_id_str), Some(addr_str)) = (args.next(), args.next()) {
        //current node being listened onto the quic-v1 defauly port with universal address for a peer node
        swarm
            .listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap())
            .unwrap();
        println!(
            "Current peerID for the peer node is {:?}",
            swarm.local_peer_id()
        );
    } else {
        //binding seed node to :8888 for the bootnode
        swarm
            .listen_on("/ip4/0.0.0.0/udp/8888/quic-v1".parse().unwrap())
            .unwrap();
        println!(
            "Current peerID for the peer node is {:?}",
            swarm.local_peer_id()
        );
    }
    swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));
    loop {
        select! {
                  event = swarm.select_next_some() => match event {
                    SwarmEvent::ConnectionEstablished { peer_id, connection_id, endpoint, num_established, concurrent_dial_errors, established_in }=>{
                        println!("{:?} {:?}",peer_id,connection_id);
                    },
                    SwarmEvent::NewListenAddr { address, listener_id } => {
                        println!("Local node is listening on {address}");
                    }
                    SwarmEvent::Behaviour(MyBehaviourEvent::Kad(kad::Event::OutboundQueryProgressed { id, result, stats, step }))=>{
                        match result {
                            QueryResult::GetClosestPeers(Ok(ok)) => {
                                println!("Got closest peers: {:?}", ok.peers);
                            }
                            QueryResult::GetClosestPeers(Err(err)) => {
                                println!("Failed to get closest peers: {err}");
                            }
                            _ => println!("Other query result: {:?}", result),
                        }
                    },
                    SwarmEvent::Behaviour(MyBehaviourEvent::Kad(kad::Event::RoutingUpdated {
                       peer,
                        is_new_peer,
                        addresses,
                       bucket_range,
                        old_peer,
                    })) => {
                        println!(
                            "Routing updated for peer: {peer}, new: {is_new_peer}, addresses: {:?}, bucket: {:?}, old_peer: {:?}",
                        addresses, bucket_range, old_peer
                        );
                   },
                    SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Sent { peer_id,..})) => {
                        println!("Sent identify info to {peer_id:?}");
                    },
                    SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Received {
                        peer_id,connection_id,info
                    })) => {
                        println!("INFO RECIEVED {:?}",info);
                        println!("{:?} is the current PROTOCOL_NAME for kad",KADPROTOCOLNAME);
                        println!("protocols at PEER SIDE ARE -- {:?}",info.protocols);
                        if info.protocols
                            .iter()
                            .any(|p| *p == KADPROTOCOLNAME)
                        {
                            for addr in info.listen_addrs {
                                println!("received addr {addr} through identify");
                                swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                            }
                        } else {
                            println!("something funky happened, investigate it");
                        }
                    }
                                SwarmEvent::Behaviour(MyBehaviourEvent::Ping(_))=>{
        println!("recieved ping from a peer");
                }
                         // SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                       //   propagation_source: peer_id,
                        //  message_id: id,
                         // message,
                     // })) => println!(
                       //       "Got message: '{}' with id: {id} from peer: {peer_id}",
                         //     String::from_utf8_lossy(&message.data),
                          //),
                      SwarmEvent::NewListenAddr { address, .. } => {
                          println!("Local node is listening on {address}");
                          let s = address;
                      }
                      _ => {println!("{:?}", event)},
                }
        }
    }
}
