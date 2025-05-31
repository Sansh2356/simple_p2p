#![allow(unused)]
use std::ops::Mul;

use futures::{AsyncRead, AsyncWrite, StreamExt};
use libp2p::{
    Multiaddr, PeerId, Swarm, SwarmBuilder, Transport,
    core::{
        muxing::StreamMuxerBox,
        transport::{Boxed, MemoryTransport, OrTransport, upgrade},
        upgrade::SelectUpgrade,
    },
    identify,
    identity::{self, Keypair},
    multiaddr::Protocol,
    noise, ping, plaintext, quic, relay,
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
#[tokio::main]
async fn main() {
    //generating the local key pair
    let local_key = Keypair::generate_ed25519();
    //generating the relay-client Transport and Network behaviour for relay-client
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
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap());
    let addr: Multiaddr = "/ip4/127.0.0.1/tcp/41043".parse().unwrap();
    let dial_addr = addr
        .with(Protocol::P2p(
            "12D3KooWA5RwKzw2zNhLaHJKiBRAM6REs6Xkc1VnEuHnPRqNiRwx"
                .parse()
                .unwrap(),
        ))
        .with(Protocol::P2pCircuit)
        .with(Protocol::P2p(
            "12D3KooWC7kBsmBbYWTydP6GsnaM8rTGS72dxnymFTicqsnoqN5i"
                .parse()
                .unwrap(),
        ));
    swarm.dial(
        "/ip4/127.0.0.1/tcp/41043/p2p/12D3KooWA5RwKzw2zNhLaHJKiBRAM6REs6Xkc1VnEuHnPRqNiRwx"
            .parse::<Multiaddr>()
            .unwrap(),
    );
    println!("{:?}", dial_addr);
    swarm.dial(dial_addr);

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
                    "Connection established with the relay successfully with the relay peerid {:?}",
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
            _ => {}
        }
    }
}

// let a:SelectUpgrade<client::Transport, tcp::Transport<_>> = SelectUpgrade::new(relay_transport, tcp::Transport::default());
// SelectUpgrade::new(a, b)
// let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
//     .with_tokio()
//     .with_tcp(
//         tcp::Config::default(),
//         noise::Config::new,
//         yamux::Config::default,
//     )
//     .unwrap()
//     .with_quic()
//     .with_relay_client(a, yamux::Config::default())
//     .with_behaviour(|key| Behaviour {
//         ping: ping::Behaviour::new(ping::Config::new()),
//         relay: behaviour,
//         identify: identify::Behaviour::new(identify::Config::new(
//             String::from("/TODO/0.0.1"),
//             key.public(),
//         )),
//     })
//     .unwrap()
//     .build();
