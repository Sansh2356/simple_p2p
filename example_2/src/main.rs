#![allow(unused)]

use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    time::Duration,
};

use futures::stream::StreamExt;
use libp2p::{
    Multiaddr, StreamProtocol,
    core::transport::ListenerId,
    kad::{
        GetClosestPeersError, GetClosestPeersOk, InboundRequest, Mode, PeerInfo, QueryResult,
        store::{MemoryStore, RecordStore},
    },
    swarm::ConnectionId,
};
use libp2p::{
    PeerId, gossipsub, identify, identity, kad, mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use tokio::{io, io::AsyncBufReadExt, select};
use tracing_subscriber::EnvFilter;

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    // mdns: mdns::tokio::Behaviour,
    kad: kad::Behaviour<MemoryStore>,
    identify: identify::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    const BOOTNODES: [&str; 1] = ["12D3KooWRHt5WVq3RhyQJBV5ofD5t8NKhf5LG1yPZ3Fg46Ct8tja"];
    // let temp_peerId = PeerId::random();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
    let mut swarm: libp2p::Swarm<MyBehaviour> = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        // .with_other_transport(constructor)
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            // To content-address message, we can take the hash of message and use it as an ID.
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            // Set a custom gossipsub configuration
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message
                // signing)
                .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
                .build()
                .map_err(io::Error::other)?; // Temporary hack because `build` does not return a proper `std::error::Error`.

            // build a gossipsub network behaviour
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;
            //custom mdns or multicast dns
            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            //key pair generation for peer id and configuration
            let id_keys = libp2p::identity::Keypair::generate_ed25519();
            //initializing the store for kademlia based DHT
            let store = MemoryStore::new(id_keys.public().to_peer_id());
            //custom kademlia protocol
            let mut kad_config = kad::Config::new(StreamProtocol::new("/ipfs/kad/1.0.0"));
            kad_config.set_query_timeout(tokio::time::Duration::from_secs(60));
            //custom kad configuration
            let kademlia_behaviour =
                kad::Behaviour::with_config(id_keys.public().to_peer_id(), store, kad_config);
            //identify protocol configuration
            let identify_config =
                identify::Config::new("Testing/0.1.0".to_string(), id_keys.public());
            let identify = identify::Behaviour::new(identify_config);
            //custom network behaviour stack from libp2p
            Ok(MyBehaviour {
                gossipsub,
                // mdns,
                kad: kademlia_behaviour,
                identify,
            })
        })?
        .build();
    let local_peerId = swarm.local_peer_id();
    println!("The local peer id is -----  {:?}", local_peerId.clone());
    //creating a test topic subscribing to the current test topic
    let current_test_topic = gossipsub::IdentTopic::new("test_topic");
    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&current_test_topic)
        .unwrap();
    //current node being listened onto the quic-v1 defauly port with universal address
    swarm
        .listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap())
        .unwrap();
    //User input message
    let mut stdin = io::BufReader::new(io::stdin()).lines();
    // while let Some(line) = stdin.next_line().await? {
    //     println!("length = {} and string is {:?}", line.len(),line);
    // }
    // SwarmEvent::Behaviour(MyBehaviourEvent::Kad(kad::Event::RoutingUpdated { peer: , is_new_peer: , addresses: , bucket_range: , old_peer:  }=>{
    //     info!("Routing updated for peer: {peer}, is_new_peer: {is_new_peer}, addresses: {addresses:?}, bucket_range: {bucket_range:?}, old_peer: {old_peer:?}");
    // }))
    //Adding the boot nodes for kademlia based DHT discoveries .

    for peer in &BOOTNODES {
        swarm
            .behaviour_mut()
            .kad
            .add_address(&peer.parse()?, "/ip4/127.0.0.1/udp/55514/quic-v1".parse()?);
    }
    swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));
    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                if let Err(e) = swarm
                    .behaviour_mut().gossipsub
                    .publish(current_test_topic.clone(), line.as_bytes()) {
                    println!("Publish error: {e:?}");
                }
            }
            event = swarm.select_next_some() => match event  {
                SwarmEvent::Behaviour(MyBehaviourEvent::Kad(kad::Event::OutboundQueryProgressed {
                result: QueryResult::GetClosestPeers(result),
                ..
            })) => {
                match result {
                    Ok(GetClosestPeersOk { key, peers }) => {
                        let a = swarm.behaviour_mut().kad.store_mut();
                        for record in a.records().into_iter(){
                            let a = record.key.clone();
                            let b = record.expires.unwrap();
                            let c = record.publisher.unwrap();
                            let d = record.value.clone();

                            println!("KEY {:?}",a);
                            println!("INSTANT {:?}",b);
                            println!("PEERID {:?}",c);
                            println!("VALUE === {:?}",d);

                        }
                        if !peers.is_empty() {
                            println!("Query finished with closest peers: {:#?}", peers);
                            for peer in peers {
                                println!("gossipsub adding peer {:?}",peer.peer_id);
                                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer.peer_id);
                            }
                        } else {
                            println!("Query finished with no closest peers.")
                        }
                    }
                    Err(GetClosestPeersError::Timeout { peers, .. }) => {
                        let a = swarm.behaviour_mut().kad.store_mut();
                        for record in a.records().into_iter(){
                            let a = record.key.clone();
                            let b = record.expires.unwrap();
                            let c = record.publisher.unwrap();
                            let d = record.value.clone();

                            println!("KEY {:?}",a);
                            println!("INSTANT {:?}",b);
                            println!("PEERID {:?}",c);
                            println!("VALUE === {:?}",d);

                        }
                        if !peers.is_empty() {
                            println!("Query timed out with closest peers: {:#?}", peers);
                            for peer in peers {
                                println!("gossipsub adding peer {:?}",peer.peer_id);
                                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer.peer_id);
                            }
                        } else {
                            println!("Query timed out with no closest peers.");
                        }
                    }
                };
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info:
                    identify::Info {
                        listen_addrs,
                        protocols,
                        ..
                    },
                    connection_id
            })) => {
                if protocols
                    .iter()
                    .any(|p| *p == kad::PROTOCOL_NAME)
                {
                    for addr in listen_addrs {
                        println!("received addr {addr} trough identify");
                        swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                    }
                } else {
                    println!("something funky happened, investigate it");
                }
            }

            SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source: peer_id,
                message_id: id,
                message,
            })) => println!(
                    "Got message: '{}'with id: {id} from peer: {peer_id}",
                    String::from_utf8_lossy(&message.data),
                ),
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Local node is listening on {address}");
            }
            _ =>{
                let a = swarm.behaviour_mut().kad.store_mut();
                for record in a.records().into_iter(){
                    let a = record.key.clone();
                    let b = record.expires.unwrap();
                    let c = record.publisher.unwrap();
                    let d = record.value.clone();

                    println!("KEY {:?}",a);
                    println!("INSTANT {:?}",b);
                    println!("PEERID {:?}",c);
                    println!("VALUE === {:?}",d);

                }
                 println!("{:?}", event)},
            }

        //     Ok(Some(line)) = stdin.next_line() => {
        //         if let Err(e) = swarm
        //             .behaviour_mut().gossipsub
        //             .publish(current_test_topic.clone(), line.as_bytes()) {
        //             println!("Publish error: {e:?}");
        //         }

        //  }
        //   event = swarm.select_next_some() => match event {
        //     SwarmEvent::ConnectionEstablished { peer_id, connection_id, endpoint, num_established, concurrent_dial_errors, established_in }=>{
        //         println!("{:?} {:?}",peer_id,connection_id);
        //     },
        //     SwarmEvent::NewListenAddr { address, listener_id } => {
        //         println!("Local node is listening on {address}");
        //     }
        //     SwarmEvent::Behaviour(MyBehaviourEvent::Kad(kad::Event::OutboundQueryProgressed { id, result, stats, step }))=>{
        //         match result {
        //             QueryResult::GetClosestPeers(Ok(ok)) => {
        //                 println!("Got closest peers: {:?}", ok.peers);
        //             }
        //             QueryResult::GetClosestPeers(Err(err)) => {
        //                 println!("Failed to get closest peers: {err}");
        //             }
        //             _ => println!("Other query result: {:?}", result),
        //         }
        //     },
        //     SwarmEvent::Behaviour(MyBehaviourEvent::Kad(kad::Event::RoutingUpdated {
        //         peer,
        //         is_new_peer,
        //         addresses,
        //         bucket_range,
        //         old_peer,
        //     })) => {
        //         println!(
        //             "Routing updated for peer: {peer}, new: {is_new_peer}, addresses: {:?}, bucket: {:?}, old_peer: {:?}",
        //             addresses, bucket_range, old_peer
        //         );
        //     },
        //     (SwarmEvent::Behaviour(MyBehaviourEvent::Identify(event))) => {
        //         println!("Identify event: {:?}", event);
        //     },
        //     SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Sent { peer_id, .. })) => {
        //         println!("Sent identify info to {peer_id:?}")
        //     },
        //     SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Received {
        //         peer_id,connection_id,info
        //     })) => {

        //         if info.protocols
        //             .iter()
        //             .any(|p| *p == kad::PROTOCOL_NAME)
        //         {
        //             for addr in info.listen_addrs {
        //                 println!("received addr {addr} through identify");
        //                 swarm.behaviour_mut().kad.add_address(&peer_id, addr);
        //             }
        //         } else {
        //             println!("something funky happened, investigate it");
        //         }
        //     }
        //     SwarmEvent::Behaviour(MyBehaviourEvent::Kad(kad::Event::InboundRequest { request }))=>{
        //         match request {
        //             InboundRequest::FindNode { num_closer_peers} => {

        //                println!("FIND NODE CALLED");
        //             },
        //             InboundRequest::AddProvider { record }=>{
        //                 println!("ADD PROVIDER CALLED");
        //             },
        //             InboundRequest::GetProvider{num_closer_peers,num_provider_peers}=>{
        //                 println!("GET PROVIDER CALLED");
        //             },
        //             InboundRequest::GetRecord{num_closer_peers,present_locally}=>{
        //                 println!("GET RECORD CALLED");
        //             },
        //             InboundRequest::PutRecord{connection,record,source}=>{
        //                 println!("PUT RECORD CALLED");
        //             }

        //     }
        // }
        //       SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
        //           propagation_source: peer_id,
        //           message_id: id,
        //           message,
        //       })) => println!(
        //               "Got message: '{}' with id: {id} from peer: {peer_id}",
        //               String::from_utf8_lossy(&message.data),
        //           ),
        //       SwarmEvent::NewListenAddr { address, .. } => {
        //           println!("Local node is listening on {address}");
        //           let s = address;
        //       }
        //       _ => {println!("{:?}", event)},
        //   }
        }
    }
    Ok(())
}
