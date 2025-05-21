use criterion::{Criterion, criterion_group, criterion_main};
use futures::prelude::*;
use libp2p::{
    Multiaddr,
    ping::{self, Behaviour, Event},
    swarm::{Swarm, SwarmEvent},
};
use std::time::Duration;
use tokio::runtime::Runtime;

fn build_swarm_quic() -> Swarm<Behaviour> {
    libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_quic()
        .with_behaviour(|_| ping::Behaviour::default())
        .unwrap()
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(30))) // Reduced from MAX for practicality
        .build()
}

fn build_swarm_tcp() -> Swarm<Behaviour> {
    libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .unwrap()
        .with_behaviour(|_| ping::Behaviour::default())
        .unwrap()
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(30)))
        .build()
}

async fn run_ping(mut swarm1: Swarm<Behaviour>, mut swarm2: Swarm<Behaviour>, proto: &str) {
    let listen_addr: Multiaddr = match proto {
        "quic" => "/ip4/127.0.0.1/udp/0/quic-v1".parse().unwrap(),
        "tcp" => "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        _ => panic!("Unknown protocol"),
    };

    swarm1.listen_on(listen_addr).expect("Failed to listen");

    let mut addr = None;
    while let Some(event) = swarm1.next().await {
        if let SwarmEvent::NewListenAddr { address, .. } = event {
            addr = Some(address);
            break;
        }
    }

    let remote = addr.expect("Failed to get listen address");
    swarm2.dial(remote).expect("Failed to dial");

    let mut ping_count = 0;
    let mut events = futures::stream::select(swarm1, swarm2);

    while let Some(event) = events.next().await {
        if let SwarmEvent::Behaviour(event) = event {
            println!("EVENT === {:?}", event);
            ping_count += 1;
            if ping_count >= 5 {
                break;
            }
        }
    }
}

fn ping_benchmark(c: &mut Criterion) {
    // let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("ping-protocols");
    group.sample_size(10); // Reduce sample size for faster benchmarks
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(10));
    group.bench_function("quic", |b| {
        b.iter(|| async {
            let s1 = build_swarm_quic();
            let s2 = build_swarm_quic();
            run_ping(s1, s2, "quic").await;
        });
    });

    group.bench_function("tcp", |b| {
        b.iter(|| async {
            let s1 = build_swarm_tcp();
            let s2 = build_swarm_tcp();
            run_ping(s1, s2, "tcp").await;
        });
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = ping_benchmark
}
criterion_main!(benches);
