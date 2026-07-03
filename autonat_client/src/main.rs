//! AutoNAT v2 **client** — determines whether *this* node is behind a NAT.
//!
//! Pairs with the `autonat_server` crate. The client dials an explicitly
//! provided server multiaddr (so it works across subnets, not only on the
//! local network the way mDNS-based discovery would), lets `identify` learn
//! the address the server observes us on, and then asks the AutoNAT v2 server
//! to dial that address back. Whether the dial-back succeeds tells us if we
//! are publicly reachable or sitting behind a NAT.
//!
//! Run the server on a publicly reachable host first:
//!
//! ```bash
//! cd autonat_server && cargo run
//! # note its PeerID and the /ip4/<public-ip>/tcp/8888 address it prints
//! ```
//!
//! Then, on the node you want to classify (a different subnet is fine):
//!
//! ```bash
//! cd autonat_client
//! cargo run -- --server /ip4/<server-public-ip>/tcp/8888/p2p/<server-peer-id>
//! ```

use std::{error::Error, net::Ipv4Addr, time::Duration};

use clap::Parser;
use libp2p::{
    Multiaddr, SwarmBuilder, autonat,
    futures::StreamExt,
    identify, identity,
    multiaddr::Protocol,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use rand::rngs::OsRng;

#[derive(Debug, Parser)]
#[command(name = "libp2p autonat v2 client")]
struct Opt {
    /// Full multiaddr of the AutoNAT v2 server, including its `/p2p/<peer-id>`.
    /// This is typically on a public IP / a different subnet than this node.
    #[arg(short, long)]
    server: Multiaddr,

    /// Local TCP port to listen on. 0 lets the OS choose a free port.
    #[arg(short, long, default_value_t = 0)]
    listen_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
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
        .with_behaviour(|key| Behaviour::new(key.public()))?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    println!("Local PeerID: {}", swarm.local_peer_id());

    // Listen on all interfaces so the server can attempt a dial-back.
    swarm.listen_on(
        Multiaddr::empty()
            .with(Ipv4Addr::UNSPECIFIED.into())
            .with(Protocol::Tcp(opt.listen_port)),
    )?;

    // Dial the AutoNAT server. Once connected and `identify` has exchanged
    // info, the server becomes a probe target for the AutoNAT v2 client.
    println!("Dialing AutoNAT server at {}", opt.server);
    swarm.dial(opt.server.clone())?;

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {address}");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("Connected to {peer_id}");
            }
            SwarmEvent::ExternalAddrConfirmed { address } => {
                println!("External address CONFIRMED reachable: {address}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                info,
                ..
            })) => {
                // The server reports the address it saw us dial from. This
                // becomes an external-address candidate that AutoNAT will probe.
                println!("Server observes us at: {}", info.observed_addr);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::v2::client::Event {
                tested_addr,
                server,
                result,
                ..
            })) => match result {
                Ok(()) => {
                    println!(
                        "\n✅ NAT STATUS: PUBLIC — server {server} successfully dialed us back \
                         on {tested_addr}. This node is directly reachable (not behind a NAT).\n"
                    );
                }
                Err(e) => {
                    println!(
                        "\n🚧 NAT STATUS: BEHIND NAT — server {server} could NOT reach us on \
                         {tested_addr} ({e}). This node is behind a NAT/firewall and needs \
                         relaying or hole-punching to be reachable.\n"
                    );
                }
            },
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                println!("Failed to connect to {peer_id:?}: {error}");
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
    pub fn new(key: identity::PublicKey) -> Self {
        Self {
            autonat: autonat::v2::client::Behaviour::new(
                OsRng,
                autonat::v2::client::Config::default()
                    // Probe promptly once we have a candidate address.
                    .with_probe_interval(Duration::from_secs(2)),
            ),
            identify: identify::Behaviour::new(identify::Config::new("/ipfs/0.1.0".into(), key)),
        }
    }
}
