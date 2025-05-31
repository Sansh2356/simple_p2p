#![allow(unused)]
use std::ops::Mul;

use clap::Parser;
use futures::{AsyncRead, AsyncWrite, StreamExt};
use libp2p::{
    Multiaddr, PeerId, Swarm, Transport,
    core::{
        muxing::StreamMuxerBox,
        transport::{Boxed, OrTransport, upgrade},
        upgrade::SelectUpgrade,
    },
    identify,
    identity::{self, Keypair},
    multiaddr::Protocol,
    noise, ping, relay,
    swarm::{Config, NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use relay::client;
#[derive(NetworkBehaviour)]
struct Behaviour {
    relay: relay::client::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}
//manuall constructing the transport stack
fn upgrade_transport<StreamSink>(
    transport: Boxed<StreamSink>,
    identity: &identity::Keypair,
) -> Boxed<(PeerId, StreamMuxerBox)>
where
    StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    transport
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::Config::new(&identity).unwrap())
        .multiplex(yamux::Config::default())
        .boxed()
}
#[derive(Parser, Debug)]
struct Args {
    /// Multiaddress to dial (relay circuit format)
    #[arg(long)]
    relay_addr: String,
}
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let local_key = Keypair::generate_ed25519();
    let (relay_transport, behaviour) = client::new(local_key.public().to_peer_id());
    //Defining the network behaviour utlizing ping,client-relay and identify
    let swarm_bheaviour = Behaviour {
        ping: ping::Behaviour::new(ping::Config::new()),
        relay: behaviour,
        identify: identify::Behaviour::new(identify::Config::new(
            String::from("/TODO/0.0.1"),
            local_key.public(),
        )),
    };
    //upgrading transport and mixing and building tcp/relay transport manually
    let transport = upgrade_transport(
        OrTransport::new(relay_transport, tcp::async_io::Transport::default()).boxed(),
        &local_key,
    );
    //building the swarm manually
    let mut swarm = Swarm::new(
        transport,
        swarm_bheaviour,
        local_key.public().to_peer_id(),
        Config::with_async_std_executor(),
    );
    // let relay_addr = "/ip4/127.0.0.1/tcp/41043".parse::<Multiaddr>().unwrap();
    // let relay_peer_id = "12D3KooWA5RwKzw2zNhLaHJKiBRAM6REs6Xkc1VnEuHnPRqNiRwx"
    //     .parse::<PeerId>()
    //     .unwrap();
    // let client_addr = relay_addr
    //     .with(Protocol::P2p(relay_peer_id))
    //     .with(Protocol::P2pCircuit);
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap());
    // swarm.listen_on(client_addr).unwrap();
    // swarm.dial("/ip4/127.0.0.1/tcp/38333".parse::<Multiaddr>().unwrap());
    // swarm.listen_on("/ip4/127.0.0.1/tcp/38333/p2p-circuit".parse().unwrap());
    // swarm.dial("/ip4/127.0.0.1/tcp/38333/".parse::<Multiaddr>().unwrap());
    let relay_addr = args.relay_addr.parse::<Multiaddr>().unwrap();
    swarm.dial(relay_addr).unwrap();

    let mut reservation_req_accepted = false;

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                println!("Relay client listenening at {:?}", address);
            }
            //after establishing the connect of a peer behing a NAT/firewall
            //checking whether the 1st phase of relay-v2 i.e. reservation via hop protocol is satisfied or not
            SwarmEvent::Behaviour(BehaviourEvent::Relay(
                client::Event::ReservationReqAccepted {
                    relay_peer_id,
                    renewal,
                    limit,
                },
            )) => {
                println!("Intial reservation has been accepted by the relay node");
            }
            SwarmEvent::Dialing {
                peer_id,
                connection_id,
            } => {
                println!("Dialing the relay node with peerId {:?}", peer_id);
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
                established_in,
            } => {
                println!(
                    "Connection established with a peer node {:?}",
                    peer_id
                );
            }
            SwarmEvent::OutgoingConnectionError {
                connection_id,
                peer_id,
                error,
            } => {
                println!(
                    "An error occurred while dialing the relay node ---- {:?}",
                    error
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {
                println!("Ping event taking place");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Sent {
                connection_id,
                peer_id,
            })) => {
                println!("Sent info to {:?}", peer_id);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                connection_id,
                peer_id,
                info,
            })) => {
                println!("Recieved info from and {:?}", info);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(
                relay::client::Event::InboundCircuitEstablished { src_peer_id, limit },
            )) => {
                println!("INBOUND CIRCUIT ESTABLISHED");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Relay(
                relay::client::Event::OutboundCircuitEstablished {
                    relay_peer_id,
                    limit,
                },
            )) => {
                println!("OUTBOUND CIRCUIT ESTABLISHED");
            }
            _ => {}
        }
    }
}
