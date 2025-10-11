use std::{
    io,
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
    time::Duration,
};

use anyhow::{Context, Result};
use either::Either;
use futures::stream::StreamExt;
use libp2p::{
    PeerId, Swarm,
    core::{
        multiaddr::{Multiaddr, Protocol},
        transport::ListenerId,
    },
    dcutr, identify, noise, ping, relay,
    swarm::{ConnectionId, NetworkBehaviour, SwarmEvent, dial_opts::DialOpts},
    tcp, yamux,
};
use serde::Serialize;

/// The redis key we push the listen client's PeerId to.
const LISTEN_CLIENT_PEER_ID: &str = "LISTEN_CLIENT_PEER_ID";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .parse_filters("info,debug,netlink_proto=warn,rustls=warn,multistream_select=warn,libp2p_core::transport::choice=off,libp2p_swarm::connection=warn,libp2p_quic=trace")
        .parse_default_env()
        .init();

    let mode = Mode::Listen;
    let transport = TransportProtocol::Tcp;

    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::new().nodelay(true),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|key, relay_client| {
            Ok(Behaviour {
                relay_client,
                identify: identify::Behaviour::new(identify::Config::new(
                    "/hole-punch-tests/1".to_owned(),
                    key.public(),
                )),
                dcutr: dcutr::Behaviour::new(key.public().to_peer_id()),
                ping: ping::Behaviour::new(
                    ping::Config::default().with_interval(Duration::from_secs(1)),
                ),
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();
    let relay_addr = match transport {
        TransportProtocol::Tcp => "/ip4/23.111.156.126/tcp/39633"
            .parse::<Multiaddr>()
            .unwrap_or_else(|_| tcp_addr(Ipv4Addr::new(127, 0, 0, 1).into()))
            .with(Protocol::P2p(
                "12D3KooWRyvVCurM3QsCK9Wg44LGWAub3BsDH7q9jFjFKYFYYdfb"
                    .parse()
                    .unwrap(),
            ))
            .with(Protocol::P2pCircuit)
            .with(Protocol::P2p(swarm.local_peer_id().clone())),
    };
    println!("{:?}", relay_addr.to_string());

    client_listen_on_transport(&mut swarm, transport).await?;
    println!("{:?}", relay_addr.to_string());

    let id = client_setup(&mut swarm, relay_addr.clone(), mode).await?;
    println!("{:?}", relay_addr.to_string());

    let mut hole_punched_peer_connection = None;
    println!("{:?}", relay_addr.to_string());

    loop {
        match (
            swarm.next().await.unwrap(),
            hole_punched_peer_connection,
            id,
        ) {
            (
                SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
                    relay::client::Event::ReservationReqAccepted { .. },
                )),
                _,
                _,
            ) => {
                println!("Relay accepted our reservation request.");
            }
            (
                SwarmEvent::Behaviour(BehaviourEvent::Dcutr(dcutr::Event {
                    remote_peer_id,
                    result: Ok(connection_id),
                })),
                _,
                _,
            ) => {
                println!("Successfully hole-punched to {remote_peer_id}");

                hole_punched_peer_connection = Some(connection_id);
            }
            (
                SwarmEvent::Behaviour(BehaviourEvent::Ping(ping::Event {
                    connection,
                    result: Ok(rtt),
                    ..
                })),
                Some(hole_punched_connection),
                _,
            ) if mode == Mode::Dial && connection == hole_punched_connection => {
                println!("{}", serde_json::to_string(&Report::new(rtt))?);

                return Ok(());
            }
            (
                SwarmEvent::Behaviour(BehaviourEvent::Dcutr(dcutr::Event {
                    remote_peer_id,
                    result: Err(error),
                    ..
                })),
                _,
                _,
            ) => {
                println!("Failed to hole-punched to {remote_peer_id}");
                return Err(anyhow::Error::new(error));
            }
            (
                SwarmEvent::ListenerClosed {
                    listener_id,
                    reason: Err(e),
                    ..
                },
                _,
                Either::Left(reservation),
            ) if listener_id == reservation => {
                anyhow::bail!("Reservation on relay failed: {e}");
            }
            (
                SwarmEvent::OutgoingConnectionError {
                    connection_id,
                    error,
                    ..
                },
                _,
                Either::Right(circuit),
            ) if connection_id == circuit => {
                anyhow::bail!("Circuit request relay failed: {error}");
            }
            e => {
                println!("None of the event matched {:?}", e);
            }
        }
    }
}

#[derive(Serialize)]
struct Report {
    rtt_to_holepunched_peer_millis: u128,
}

impl Report {
    fn new(rtt: Duration) -> Self {
        Self {
            rtt_to_holepunched_peer_millis: rtt.as_millis(),
        }
    }
}

async fn client_listen_on_transport(
    swarm: &mut Swarm<Behaviour>,
    transport: TransportProtocol,
) -> Result<()> {
    let listen_addr = match transport {
        TransportProtocol::Tcp => tcp_addr(Ipv4Addr::UNSPECIFIED.into()),
    };
    let expected_listener_id = swarm
        .listen_on(listen_addr)
        .context("Failed to listen on address")?;

    let mut listen_addresses = 0;

    // We should have at least two listen addresses, one for localhost and the actual interface.
    while listen_addresses < 2 {
        if let SwarmEvent::NewListenAddr {
            listener_id,
            address,
        } = swarm.next().await.unwrap()
        {
            if listener_id == expected_listener_id {
                listen_addresses += 1;
            }

            println!("Listening on {address}");
        }
    }
    Ok(())
}

async fn client_setup(
    swarm: &mut Swarm<Behaviour>,
    relay_addr: Multiaddr,
    mode: Mode,
) -> Result<Either<ListenerId, ConnectionId>> {
    let either = match mode {
        Mode::Listen => {
            let id = swarm.listen_on(relay_addr)?;

            Either::Left(id)
        }
        Mode::Dial => {
            let remote_peer_id = LISTEN_CLIENT_PEER_ID.parse::<PeerId>().unwrap();

            let opts = DialOpts::from(
                relay_addr
                    .with(Protocol::P2pCircuit)
                    .with(Protocol::P2p(remote_peer_id)),
            );
            let id = opts.connection_id();

            swarm.dial(opts)?;

            Either::Right(id)
        }
    };

    Ok(either)
}

fn tcp_addr(addr: IpAddr) -> Multiaddr {
    Multiaddr::empty().with(addr.into()).with(Protocol::Tcp(0))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TransportProtocol {
    Tcp,
}

impl FromStr for TransportProtocol {
    type Err = io::Error;
    fn from_str(mode: &str) -> Result<Self, Self::Err> {
        match mode {
            "tcp" => Ok(TransportProtocol::Tcp),
            _ => Err(io::Error::other("Expected either 'tcp' or 'quic'")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Dial,
    Listen,
}

impl FromStr for Mode {
    type Err = io::Error;
    fn from_str(mode: &str) -> Result<Self, Self::Err> {
        match mode {
            "dial" => Ok(Mode::Dial),
            "listen" => Ok(Mode::Listen),
            _ => Err(io::Error::other("Expected either 'dial' or 'listen'")),
        }
    }
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    relay_client: relay::client::Behaviour,
    identify: identify::Behaviour,
    dcutr: dcutr::Behaviour,
    ping: ping::Behaviour,
}
