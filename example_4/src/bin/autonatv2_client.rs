#![allow(unused)]
use std::{error::Error, net::Ipv4Addr, time::Duration};
use clap::Parser;
use libp2p::{
    Multiaddr, StreamProtocol, SwarmBuilder, autonat,
    futures::StreamExt,
    identify, identity,
    multiaddr::Protocol,
    noise,
    request_response::{self, ProtocolSupport},
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
        .with_behaviour(|key| Behaviour::new(key.public(), opt.probe_interval))?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(10)))
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
            _ => {}
        }
    }
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    autonat: autonat::v2::client::Behaviour,
    identify: identify::Behaviour,
    request_response: request_response::cbor::Behaviour<GreetRequest, GreetResponse>,
}

impl Behaviour {
    pub fn new(key: identity::PublicKey, probe_interval: u64) -> Self {
        Self {
            autonat: autonat::v2::client::Behaviour::new(
                OsRng,
                autonat::v2::client::Config::default()
                    .with_probe_interval(Duration::from_secs(probe_interval)),
            ),
            identify: identify::Behaviour::new(identify::Config::new("/ipfs/0.1.0".into(), key)),
            request_response: request_response::cbor::Behaviour::new(
                [(StreamProtocol::new("/test/1"), ProtocolSupport::Full)],
                request_response::Config::default(),
            ),
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
