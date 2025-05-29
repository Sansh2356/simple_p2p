// #![allow(unused)]

// use futures::stream::StreamExt;
// use libp2p::{
//     Multiaddr, StreamProtocol,
//     core::transport::ListenerId,
//     kad::{
//         GetClosestPeersError, GetClosestPeersOk, InboundRequest, Mode, PeerInfo, QueryResult,
//         store::{MemoryStore, RecordStore},
//     },
//     swarm::ConnectionId,
// };
// use libp2p::{
//     PeerId, autonat, gossipsub, identify, identity, kad, mdns, noise,
//     swarm::{NetworkBehaviour, SwarmEvent},
//     tcp, yamux,
// };
// use rand::rngs::OsRng;
// use std::{
//     collections::hash_map::DefaultHasher,
//     error::Error,
//     hash::{Hash, Hasher},
//     net::Ipv4Addr,
//     time::Duration,
// };
// use tokio::{io, io::AsyncBufReadExt, select};
// use tracing_subscriber::EnvFilter;

// // We create a custom network behaviour that combines Gossipsub and Mdns.
// #[derive(NetworkBehaviour)]
// struct MyBehaviour {
//     gossipsub: gossipsub::Behaviour,
//     // mdns: mdns::tokio::Behaviour,
//     kad: kad::Behaviour<MemoryStore>,
//     identify: identify::Behaviour,
//     autonat: autonat::v2::client::Behaviour,
// }

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     // let temp_peerId = PeerId::random();
//     let _ = tracing_subscriber::fmt()
//         .with_env_filter(EnvFilter::from_default_env())
//         .try_init();
//     let mut swarm: libp2p::Swarm<MyBehaviour> = libp2p::SwarmBuilder::with_new_identity()
//         .with_tokio()
//         // .with_other_transport(constructor)
//         .with_tcp(
//             tcp::Config::default(),
//             noise::Config::new,
//             yamux::Config::default,
//         )?
//         .with_quic()
//         .with_behaviour(|key| {
//             // To content-address message, we can take the hash of message and use it as an ID.
//             let message_id_fn = |message: &gossipsub::Message| {
//                 let mut s = DefaultHasher::new();
//                 message.data.hash(&mut s);
//                 gossipsub::MessageId::from(s.finish().to_string())
//             };

//             // Set a custom gossipsub configuration
//             let gossipsub_config = gossipsub::ConfigBuilder::default()
//                 .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
//                 .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message
//                 // signing)
//                 .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
//                 .build()
//                 .map_err(io::Error::other)?; // Temporary hack because `build` does not return a proper `std::error::Error`.

//             // build a gossipsub network behaviour
//             let gossipsub = gossipsub::Behaviour::new(
//                 gossipsub::MessageAuthenticity::Signed(key.clone()),
//                 gossipsub_config,
//             )?;
//             //custom mdns or multicast dns
//             let mdns =
//                 mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
//             //key pair generation for peer id and configuration
//             let id_keys = libp2p::identity::Keypair::generate_ed25519();
//             //initializing the store for kademlia based DHT
//             let store = MemoryStore::new(id_keys.public().to_peer_id());
//             //custom kademlia protocol
//             let mut kad_config = kad::Config::new(StreamProtocol::new("/ipfs/kad/1.0.0"));
//             kad_config.set_query_timeout(tokio::time::Duration::from_secs(60));
//             //custom kad configuration
//             let kademlia_behaviour =
//                 kad::Behaviour::with_config(id_keys.public().to_peer_id(), store, kad_config);
//             //identify protocol configuration
//             let identify_config =
//                 identify::Config::new("Testing/0.1.0".to_string(), id_keys.public());
//             let identify = identify::Behaviour::new(identify_config);
//             //custom network behaviour stack from libp2p
//             let autonat_client_config = autonat::v2::client::Config::default().with_probe_interval(Duration::from_secs(2));
//             let autonat_client = autonat::v2::client::Behaviour::new(OsRng, autonat_client_config);
//             Ok(MyBehaviour {
//                 gossipsub,
//                 kad: kademlia_behaviour,
//                 identify,
//                 autonat: autonat_client,
//             })
//         })?
//         .build();
//     swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?);
//     println!("Current peerID is {:?}",swarm.local_peer_id());
//     //dialing the autonat-server
//     swarm.dial("/ip4/127.0.0.1/tcp/8888".parse::<Multiaddr>().unwrap());
//     println!("Dialed server IP");
//     loop {
//         match swarm.select_next_some().await {
//             SwarmEvent::NewListenAddr { address, .. } => {
//                 println!("Listening on {address:?}");
//             }
//             SwarmEvent::Behaviour(MyBehaviourEvent::Autonat(autonat::v2::client::Event {
//                 server,
//                 tested_addr,
//                 bytes_sent,
//                 result: Ok(()),
//             })) => {
//                 println!(
//                     "Tested {tested_addr} with {server}. Sent {bytes_sent} bytes for verification. Everything Ok and verified."
//                 );
//             }
//             SwarmEvent::Behaviour(MyBehaviourEvent::Autonat(autonat::v2::client::Event {
//                 server,
//                 tested_addr,
//                 bytes_sent,
//                 result: Err(e),
//             })) => {
//                 println!(
//                     "Tested {tested_addr} with {server}. Sent {bytes_sent} bytes for verification. Failed with {e:?}."
//                 );
//             }
//             SwarmEvent::ExternalAddrConfirmed { address } => {
//                 println!("External address confirmed: {address}");
//             }
//             _ => {}
//         }
//     }
//     Ok(())
// }

use std::{error::Error, net::Ipv4Addr, time::Duration};

use clap::Parser;
use libp2p::{
    Multiaddr, SwarmBuilder, autonat,
    futures::StreamExt,
    identify, identity,
    multiaddr::Protocol,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent, dial_opts::DialOpts},
    tcp, yamux,
};
use rand::rngs::OsRng;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "libp2p autonatv2 client")]
struct Opt {
    /// Port where the client will listen for incoming connections.
    #[arg(short = 'p', long, default_value_t = 0)]
    listen_port: u16,

    /// Address of the server where want to connect to.
    #[arg(short = 'a', long)]
    server_address: Multiaddr,

    /// Probe interval in seconds.
    #[arg(short = 't', long, default_value = "2")]
    probe_interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let opt = Opt::parse();

    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_dns()?
        .with_behaviour(|key| Behaviour::new(key.public(), 1))?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(1)))
        .build();

    swarm.listen_on(
        Multiaddr::empty()
            .with(Protocol::Ip4(Ipv4Addr::UNSPECIFIED))
            .with(Protocol::Tcp(opt.listen_port)),
    )?;

    swarm.dial(
        DialOpts::unknown_peer_id()
            .address(opt.server_address)
            .build(),
    )?;
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {address:?}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::v2::client::Event {
                server,
                tested_addr,
                bytes_sent,
                result: Ok(()),
            })) => {
                println!(
                    "Tested {tested_addr} with {server}. Sent {bytes_sent} bytes for verification. Everything Ok and verified."
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::v2::client::Event {
                server,
                tested_addr,
                bytes_sent,
                result: Err(e),
            })) => {
                println!(
                    "Tested {tested_addr} with {server}. Sent {bytes_sent} bytes for verification. Failed with {e:?}."
                );
            }
            SwarmEvent::ExternalAddrConfirmed { address } => {
                println!("External address confirmed: {address}");
            }
            _ => {}
        }
    }
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    autonat: autonat::v2::client::Behaviour,
    identify: identify::Behaviour,
}

impl Behaviour {
    pub fn new(key: identity::PublicKey, probe_interval: u64) -> Self {
        Self {
            autonat: autonat::v2::client::Behaviour::new(
                OsRng,
                autonat::v2::client::Config::default()
                    .with_probe_interval(Duration::from_millis(100)),
            ),
            identify: identify::Behaviour::new(identify::Config::new("/ipfs/0.1.0".into(), key)),
        }
    }
}
