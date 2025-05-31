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
    bytes::Bytes,
    core::transport::ListenerId,
    floodsub::{Floodsub, FloodsubEvent, FloodsubMessage},
    kad::{Mode, QueryResult, store::MemoryStore},
    swarm::behaviour,
};
use libp2p::{
    PeerId, floodsub, gossipsub, identify, identity, kad, mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use tokio::{io, io::AsyncBufReadExt, select};
use tracing_subscriber::EnvFilter;

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    // gossipsub: gossipsub::Behaviour,
    // mdns: mdns::tokio::Behaviour,
    floodsub: floodsub::Floodsub,
    kad: kad::Behaviour<MemoryStore>,
    identify: identify::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    const KADPROTOCOLNAME: StreamProtocol = StreamProtocol::new("/braidpool/kad/1.0.0");
    const IDENTIFYPROTOCOLNAME: StreamProtocol = StreamProtocol::new("/braidpool/identify/1.0.0");
    const FLOODSUBPROTOCOLNAME: StreamProtocol = StreamProtocol::new("/floodsub/1.0.0");
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
            // let a = identify_config
            //     .clone()
            //     .with_interval(Duration::from_secs(1800));
            let identify = identify::Behaviour::new(identify_config);
            //custom network behaviour stack from libp2p
            let flood_sub_config = floodsub::FloodsubConfig::new(key.public().to_peer_id());
            let floodsub = floodsub::Floodsub::from_config(flood_sub_config);
            Ok(MyBehaviour {
                // gossipsub,
                // mdns,
                kad: kademlia_behaviour,
                identify,
                floodsub: floodsub,
            })
        })?
        .build();

    //creating a test topic subscribing to the current test topic
    let current_test_topic = floodsub::Topic::new("test_topic");
    let topic_ref = current_test_topic.clone();

    swarm.behaviour_mut().floodsub.subscribe(current_test_topic);
    let mut args = env::args().skip(1);
    swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));
    if let (Some(peer_id_str), Some(addr_str)) = (args.next(), args.next()) {
        //current node being listened onto the quic-v1 defauly port with universal address for a peer node
        swarm
            .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
            .unwrap();
        println!(
            "Current peerID for the peer node is {:?}",
            swarm.local_peer_id()
        );
        // Parse the peer ID and address
        let peer_id: PeerId = peer_id_str.parse()?;
        let addr: Multiaddr = addr_str.parse()?;
        let addr_reference = addr.clone();
        let boot_nodes: [&PeerId; 1] = [&peer_id];
        //Adding the boot nodes for kademlia based DHT discoveries .
        for peer in &boot_nodes {
            swarm
                .behaviour_mut()
                .kad
                .add_address(&peer, addr_reference.clone());
        }
        swarm.dial(addr_reference)?;
        println!("Dialed bootNode")
    } else {
        //binding seed node to :8888 for the bootnode
        swarm
            .listen_on("/ip4/0.0.0.0/tcp/8888".parse().unwrap())
            .unwrap();
        println!(
            "Current peerID for the peer node is {:?}",
            swarm.local_peer_id()
        );
    }
    let mut stdin = io::BufReader::new(io::stdin()).lines();
    loop {
        select! {
        Ok(Some(line)) = stdin.next_line() => {
            let a = line.clone();
            swarm.behaviour_mut().floodsub.publish(topic_ref.clone(),a)
        }
        event = swarm.select_next_some() => match event {
             SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(floodsub::FloodsubEvent::Subscribed { peer_id, topic }))=>{
            println!("A new peer {:?} subscribed to the topic {:?}",peer_id,topic);
        }
        SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(floodsub::FloodsubEvent::Unsubscribed { peer_id, topic }))=>{
            println!("A peer {:?} unsubsribed from the topic {:?}",peer_id,topic);
        }
        SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(floodsub::FloodsubEvent::Message(message)))=>{
            println!("{:?} Message has been recieved  from the peer {:?} and having data {:?}",message.topics,message.source,message.data);
        }
        SwarmEvent::ConnectionEstablished { peer_id, connection_id, endpoint, num_established, concurrent_dial_errors, established_in }=>{
            println!("{:?} {:?}",peer_id,connection_id);
        },
            SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Sent { peer_id,..})) => {
                println!("Sent identify info to {peer_id:?}");
            },
            SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Received {
                peer_id,connection_id,info
            })) => {
                 println!("INFO RECIEVED {:?}",info);
                let protocol_ref = info.clone();
                if info.protocols
                    .iter()
                    .any(|p| *p == "/braidpool/kad/1.0.0")
                {
                    for addr in info.listen_addrs {
                        swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                    }
                } else {
                    println!("something funky happened, investigate it while updating the routing table KAD");
                }
                if protocol_ref.protocols.iter()
                .any(|p| *p == "/floodsub/1.0.0")
            {
                for addr in protocol_ref.listen_addrs {
                   swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                }
            } else {
                println!("something funky happened, investigate it FLOODSUB");
            }
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

            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Local node is listening on {address}");
            }
            
            _ => {println!("{:?}",event);}
        }

        }
    }
    Ok(())
}
