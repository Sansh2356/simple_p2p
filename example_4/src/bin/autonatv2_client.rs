#![allow(unused)]
use clap::Parser;
use libp2p::swarm::DialError;
use libp2p::{
    Multiaddr, StreamProtocol, SwarmBuilder, autonat,
    futures::StreamExt,
    identify, identity,
    multiaddr::Protocol,
    noise,
    request_response::{self, ProtocolSupport},
    swarm::{self, NetworkBehaviour, SwarmEvent, dial_opts::DialOpts},
    tcp, yamux,
};
use rand::rngs::OsRng;
use std::{error::Error, net::Ipv4Addr, time::Duration};
use tracing_subscriber::EnvFilter;
#[derive(Debug, Parser)]
#[command(name = "libp2p autonatv2 client")]
struct Opt {
    /// Port where the client will listen for incoming connections.
    #[arg(short = 'p', long, default_value_t = 0)]
    listen_port: u16,

    // /// Address of the server where want to connect to.
    // #[arg(short = 'a', long)]
    // server_address: Multiaddr,
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
        .with_behaviour(|key| Behaviour::new(key.public(), opt.probe_interval))?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(10)))
        .build();
    swarm.listen_on("/ip4/0.0.0.0/tcp/8282".parse::<Multiaddr>().unwrap())?;
    // swarm.dial(
    //     DialOpts::unknown_peer_id()
    //         .address(opt.server_address)
    //         .build(),
    // )?;
    // autonat::v2::client::Behaviour::default()
    // DialBackResponse
    swarm.behaviour_mut().autonat.add_server(
        "12D3KooWJLRHBNu6p8FwGsgijRk9DaNDmrmCKgGSr3wmJFFZVoxC"
            .parse()
            .unwrap(),
        Some("/ip4/127.0.0.1/tcp/8283".parse::<Multiaddr>().unwrap()),
    );
    loop {
        match swarm.select_next_some().await {
            // SwarmEvent::Dialing { peer_id, connection_id }=>{
            //     println!("DIALING PEER WITH PEER ID {:?}",peer_id);
            // }
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(event)) => {
                println!("AUTONAT EVENT {:?}", event);
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {address:?}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::Event::InboundProbe(
                inbound,
            ))) => match inbound {
                autonat::InboundProbeEvent::Request {
                    probe_id,
                    peer,
                    addresses,
                } => {
                    println!("{:?} {:?} {:?}", probe_id, peer, addresses);
                }
                autonat::InboundProbeEvent::Response {
                    probe_id,
                    peer,
                    address,
                } => {
                    println!("{:?} {:?} {:?}", probe_id, peer, address);
                }
                autonat::InboundProbeEvent::Error {
                    probe_id,
                    peer,
                    error,
                } => {
                    println!("{:?} {:?} {:?}", probe_id, peer, error);
                }
            },
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::Event::OutboundProbe(
                outbound,
            ))) => match outbound {
                autonat::OutboundProbeEvent::Error {
                    probe_id,
                    peer,
                    error,
                } => {
                    println!("{:?} {:?} {:?}", probe_id, peer, error);
                }
                autonat::OutboundProbeEvent::Request { probe_id, peer } => {
                    println!("{:?} {:?}", probe_id, peer);
                }
                autonat::OutboundProbeEvent::Response {
                    probe_id,
                    peer,
                    address,
                } => {
                    println!("{:?} {:?} {:?}", probe_id, peer, address);
                }
            },
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::Event::StatusChanged {
                old,
                new,
            })) => {
                println!("STATUS CHANGED OCCURRED {:?} {:?}", old, new);
            }
            // SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::v2::client::Event {
            //     server,
            //     tested_addr,
            //     bytes_sent,
            //     result: Ok(()),
            // })) => {
            //     println!(
            //         "Tested {tested_addr} with {server}. Sent {bytes_sent} bytes for verification. Everything Ok and verified."
            //     );
            // }
            // SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::v2::client::Event {
            //     server,
            //     tested_addr,
            //     bytes_sent,
            //     result: Err(e),
            // })) => {
            //     println!(
            //         "Tested {tested_addr} with {server}. Sent {bytes_sent} bytes for verification. Failed with {e:?}."
            //     );
            // }
            SwarmEvent::ExternalAddrConfirmed { address } => {
                println!("External address confirmed: {address}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
                request_response::Event::Message {
                    peer,
                    connection_id,
                    message,
                },
            )) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    println!("{:?} request and channel {:?}", request, channel);
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    println!("{:?} request_id and response {:?}", request_id, response);
                }
            },
            event => {
                println!("{:?}", event);
            }
        }
    }
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    autonat: autonat::Behaviour,
    identify: identify::Behaviour,
    request_response: request_response::cbor::Behaviour<GreetRequest, GreetResponse>,
}

impl Behaviour {
    pub fn new(key: identity::PublicKey, probe_interval: u64) -> Self {
        let key_ref = key.clone();
        Self {
            // autonat: autonat::v2::client::Behaviour::new(
            //     OsRng,
            //     autonat::v2::client::Config::default()
            //         .with_probe_interval(Duration::from_secs(probe_interval)),
            // ),
            identify: identify::Behaviour::new(identify::Config::new("/ipfs/0.1.0".into(), key)),
            request_response: request_response::cbor::Behaviour::new(
                [(StreamProtocol::new("/test/1"), ProtocolSupport::Full)],
                request_response::Config::default(),
            ),
            autonat: autonat::Behaviour::new(key_ref.to_peer_id(), autonat::Config::default()),
        }
    }
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GreetRequest {
    name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GreetResponse {
    message: String,
}
