//! AutoNAT v2 **client**.
//!
//! Implements the client role of the AutoNAT v2 spec (`autonat-v2.md`):
//!
//!  * Sends `DialRequest` messages (a priority-ordered list of address
//!    candidates + a nonce) to connected AutoNAT servers.
//!  * Answers the server's dial-back on `/libp2p/autonat/2/dial-back` and, when
//!    the server asks for it, pays the amplification-prevention data cost.
//!  * Verifies the returned nonce matches the one it sent.
//!
//! All of that lives inside `autonat::v2::client::Behaviour`. Address
//! candidates are fed to it automatically from `identify`'s observed addresses
//! (`NewExternalAddrCandidate`), and every connected server is used as a
//! verifier.
//!
//! On top of the raw per-probe events this binary tracks, per address, how many
//! distinct servers reported it reachable vs unreachable and applies the spec's
//! suggested heuristic (§Implementation Suggestions):
//!
//!  * `> 3` servers report success  → address is **publicly reachable**.
//!  * `> 3` servers report failure  → address is **unreachable / behind NAT**.
//!
//! and prints a clear "PUBLICLY REACHABLE" / "BEHIND NAT" verdict.

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    net::Ipv4Addr,
    time::Duration,
};

use clap::Parser;
use libp2p::{
    Multiaddr, PeerId, SwarmBuilder, autonat,
    futures::StreamExt,
    identify, identity,
    multiaddr::Protocol,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent, dial_opts::DialOpts},
    tcp, yamux,
};
use rand::rngs::OsRng;
use tracing_subscriber::EnvFilter;

/// Number of *distinct* servers that must agree before we treat a verdict as
/// final. Straight from the spec's suggested heuristic.
const CONFIRMATION_THRESHOLD: usize = 3;

#[derive(Debug, Parser)]
#[command(name = "libp2p autonatv2 client")]
struct Opt {
    /// Port where the client will listen for incoming connections.
    #[arg(short = 'p', long, default_value_t = 0)]
    listen_port: u16,

    /// Address of the server we want to test our reachability against.
    #[arg(short = 'a', long)]
    server_address: Multiaddr,

    /// Probe interval in seconds.
    #[arg(short = 't', long, default_value = "2")]
    probe_interval: u64,
}

/// Per-address tally of which servers reported it reachable / unreachable,
/// plus the last verdict we printed (so we only announce changes).
#[derive(Default)]
struct AddrStats {
    reachable_by: HashSet<PeerId>,
    unreachable_by: HashSet<PeerId>,
    announced: Option<Reachability>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Reachability {
    Public,
    BehindNat,
}

/// Returns `true` if `addr` is a private/loopback address that, per the spec,
/// a client SHOULD NOT submit for public reachability testing (RFC 1918 etc.).
fn is_private(addr: &Multiaddr) -> bool {
    addr.iter().any(|p| match p {
        Protocol::Ip4(ip) => ip.is_private() || ip.is_loopback() || ip.is_unspecified(),
        Protocol::Ip6(ip) => ip.is_loopback() || ip.is_unspecified(),
        _ => false,
    })
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
            .address(opt.server_address.clone())
            .build(),
    )?;

    println!("== AutoNAT v2 CLIENT ==");
    println!("Local peer id: {}", swarm.local_peer_id());
    println!("Testing reachability against server: {}", opt.server_address);
    println!(
        "Heuristic: an address is decided once {CONFIRMATION_THRESHOLD}+ servers agree \
         (spec §Implementation Suggestions)."
    );

    // Reachability tally, keyed by the address that was tested.
    let mut stats: HashMap<Multiaddr, AddrStats> = HashMap::new();

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("[listen] Listening on {address}");
            }

            // A candidate external address (typically learned via identify's
            // observed address) that the behaviour will try to verify.
            SwarmEvent::NewExternalAddrCandidate { address } => {
                if is_private(&address) {
                    println!(
                        "[candidate] {address} is private (RFC 1918) — spec says clients \
                         SHOULD NOT test private addresses; the server SHOULD refuse it."
                    );
                } else {
                    println!("[candidate] New address to verify: {address}");
                }
            }

            // The server confirmed this address is publicly reachable.
            SwarmEvent::ExternalAddrConfirmed { address } => {
                println!("[confirmed] ✅ External address CONFIRMED reachable: {address}");
            }

            // One completed AutoNAT v2 probe against one server.
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::v2::client::Event {
                server,
                tested_addr,
                bytes_sent,
                result,
            })) => {
                let entry = stats.entry(tested_addr.clone()).or_default();
                match &result {
                    Ok(()) => {
                        entry.unreachable_by.remove(&server);
                        entry.reachable_by.insert(server);
                        println!(
                            "[probe] ✅ {tested_addr} reachable — verified by {server} \
                             ({bytes_sent} bytes sent for amplification prevention)."
                        );
                    }
                    Err(e) => {
                        entry.reachable_by.remove(&server);
                        entry.unreachable_by.insert(server);
                        println!(
                            "[probe] ❌ {tested_addr} NOT reachable via {server} \
                             ({bytes_sent} bytes sent). Reason: {e}"
                        );
                    }
                }

                let ok = entry.reachable_by.len();
                let bad = entry.unreachable_by.len();
                println!("        tally for {tested_addr}: {ok} reachable / {bad} unreachable");

                // Apply the spec's confirmation heuristic and only announce
                // when the verdict first becomes final or flips.
                let verdict = if ok >= CONFIRMATION_THRESHOLD {
                    Some(Reachability::Public)
                } else if bad >= CONFIRMATION_THRESHOLD {
                    Some(Reachability::BehindNat)
                } else {
                    None
                };

                if let Some(v) = verdict {
                    if entry.announced != Some(v) {
                        entry.announced = Some(v);
                        println!("════════════════════════════════════════════");
                        match v {
                            Reachability::Public => println!(
                                "  🌍 VERDICT: {tested_addr}\n     → PUBLICLY REACHABLE — you are \
                                 NOT behind a NAT/firewall on this address."
                            ),
                            Reachability::BehindNat => println!(
                                "  🔒 VERDICT: {tested_addr}\n     → BEHIND NAT / FIREWALL — this \
                                 address is not publicly dialable. Consider using a relay."
                            ),
                        }
                        println!("════════════════════════════════════════════");
                    }
                }
            }

            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("[conn] Connected to potential AutoNAT server {peer_id}");
            }

            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                println!("[conn] Failed to connect to {peer_id:?}: {error}");
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
                    .with_probe_interval(Duration::from_secs(probe_interval)),
            ),
            identify: identify::Behaviour::new(identify::Config::new("/ipfs/0.1.0".into(), key)),
        }
    }
}
