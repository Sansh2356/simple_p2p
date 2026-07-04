//! AutoNAT v2 **server**.
//!
//! Implements the server role of the AutoNAT v2 spec (`autonat-v2.md`):
//!
//!  * Listens on `/libp2p/autonat/2/dial-request` for `DialRequest` messages.
//!  * Selects the first address it is willing to dial from the client's
//!    priority-ordered list and dials back over `/libp2p/autonat/2/dial-back`
//!    with the request nonce.
//!  * When the selected address has a different IP than the client's observed
//!    IP, runs the amplification-attack-prevention handshake, asking the client
//!    to transfer 30k-100k bytes before dialing (see spec §Amplification Attack
//!    Prevention).
//!
//! All of that protocol machinery lives inside `autonat::v2::server::Behaviour`.
//! This binary drives the swarm and turns each completed probe into a clear log
//! line describing whether the *client* address we tested is publicly reachable
//! (i.e. the client is NOT behind a NAT on that address) or not.

use std::{error::Error, net::Ipv4Addr, time::Duration};

use cfg_if::cfg_if;
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
#[command(name = "libp2p autonatv2 server")]
struct Opt {
    #[arg(short, long, default_value_t = 0)]
    listen_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    cfg_if! {
        if #[cfg(feature = "jaeger")] {
            use opentelemetry::trace::TracerProvider as _;
            use opentelemetry::KeyValue;
            use opentelemetry_otlp::SpanExporter;
            use opentelemetry_sdk::{runtime, trace::TracerProvider};
            use tracing_subscriber::layer::SubscriberExt;

            let provider = TracerProvider::builder()
                .with_batch_exporter(
                    SpanExporter::builder().with_tonic().build()?,
                    runtime::Tokio,
                )
                .with_resource(opentelemetry_sdk::Resource::new(vec![KeyValue::new(
                    "service.name",
                    "autonatv2",
                )]))
                .build();
            let telemetry = tracing_opentelemetry::layer()
                .with_tracer(provider.tracer("autonatv2"));
            let subscriber = tracing_subscriber::Registry::default()
                .with(telemetry);
        } else {
            let subscriber = tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish();
        }
    }
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

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
        // Bound every dial (including AutoNAT dial-backs) so that probing a
        // NATed/firewalled client address fails in ~10s instead of hanging on
        // TCP retransmits for tens of seconds. This makes the "not reachable"
        // verdict surface quickly rather than flooding the log with retries.
        .with_connection_timeout(Duration::from_secs(10))
        .build();

    swarm.listen_on(
        Multiaddr::empty()
            .with(Protocol::Ip4(Ipv4Addr::UNSPECIFIED))
            .with(Protocol::Tcp(opt.listen_port)),
    )?;

    println!("== AutoNAT v2 SERVER ==");
    println!("Local peer id: {}", swarm.local_peer_id());

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("[listen] Ready to serve dial-requests on {address}");
            }

            // A client asked us to verify one of its addresses. The behaviour
            // has already selected an address, (optionally) run amplification
            // prevention, dialed back with the nonce, and produced a result.
            SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::v2::server::Event {
                all_addrs,
                tested_addr,
                client,
                data_amount,
                result,
            })) => {
                println!("──────────────────────────────────────────────");
                println!("[probe] Client {client} requested a reachability test");
                println!("        candidate addresses ({}): {all_addrs:?}", all_addrs.len());
                println!("        selected & dialed:  {tested_addr}");
                if data_amount > 0 {
                    // Non-zero only when the tested IP differs from the client's
                    // observed IP, i.e. amplification-attack prevention kicked in.
                    println!(
                        "        amplification guard: required client to send {data_amount} bytes \
                         before dialing (spec §Amplification Attack Prevention)"
                    );
                }
                match result {
                    Ok(()) => {
                        // Dial-back succeeded and the nonce came back: the client
                        // accepts inbound connections on this address.
                        println!(
                            "        RESULT: ✅ REACHABLE — dial-back succeeded, nonce verified."
                        );
                        println!(
                            "        VERDICT: client is PUBLICLY REACHABLE on {tested_addr} \
                             (NOT behind a NAT/firewall for this address)."
                        );
                    }
                    Err(e) => {
                        // We could not complete the dial-back to the client.
                        println!("        RESULT: ❌ NOT REACHABLE — dial-back failed: {e}");
                        println!(
                            "        VERDICT: client appears to be BEHIND A NAT/FIREWALL on \
                             {tested_addr} (address not publicly dialable)."
                        );
                    }
                }
                println!("──────────────────────────────────────────────");
            }

            // Identify tells us the client's observed (public) address and its
            // advertised listen addresses — useful context for what we dial back.
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            })) => {
                println!(
                    "[identify] {peer_id} observed_addr={} listen_addrs={:?}",
                    info.observed_addr, info.listen_addrs
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(_)) => {}

            SwarmEvent::IncomingConnection { send_back_addr, .. } => {
                println!("[conn] Incoming connection from {send_back_addr}");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("[conn] Connection established with {peer_id}");
            }

            // Start of a dial-back attempt to a client we are testing. Kept quiet
            // (one line) so the eventual verdict block stands out.
            SwarmEvent::Dialing {
                peer_id: Some(peer),
                ..
            } => {
                println!("[dial-back] → attempting to reach client {peer} on a fresh connection…");
            }

            // A dial-back that did not complete. For a NATed/firewalled client
            // this is expected: the address is not publicly dialable. The
            // authoritative verdict is still the `[probe]` block emitted by the
            // AutoNAT behaviour once it gives up.
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                println!(
                    "[dial-back] ✗ could not reach {peer_id:?}: {error} \
                     (address not publicly reachable — client likely behind NAT)"
                );
            }

            // Everything else (listener housekeeping, external-addr candidates,
            // connection closes, …) is not essential to the reachability story.
            _ => {}
        }
    }
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    autonat: autonat::v2::server::Behaviour,
    identify: identify::Behaviour,
}

impl Behaviour {
    pub fn new(key: identity::PublicKey) -> Self {
        Self {
            autonat: autonat::v2::server::Behaviour::new(OsRng),
            identify: identify::Behaviour::new(identify::Config::new("/ipfs/0.1.0".into(), key)),
        }
    }
}
