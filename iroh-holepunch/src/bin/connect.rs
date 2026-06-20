//! Connect side of the iroh hole-punching example.
//!
//! Dials the `EndpointId` printed by the `listen` binary and then keeps a
//! steady trickle of echo traffic going for a while, printing how the
//! connection's path evolves. On the first connect the path is typically
//! `RELAY`; once the peers hole-punch, it upgrades to `DIRECT`.
//!
//! Run with:
//! ```bash
//! cargo run --bin connect -- <endpoint-id> [seconds]
//! ```

use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use iroh::{Endpoint, EndpointId, endpoint::presets};
use iroh_holepunch::{ALPN, report_paths};

#[derive(Parser)]
struct Args {
    /// The `EndpointId` printed by the `listen` binary.
    endpoint_id: EndpointId,
    /// How long to keep the connection alive while watching for a direct path.
    #[arg(default_value_t = 30)]
    seconds: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let ep = Endpoint::bind(presets::N0).await?;
    println!("connecting to {}...", args.endpoint_id);

    let conn = ep.connect(args.endpoint_id, ALPN).await?;
    println!("connected; watching path for {}s...", args.seconds);

    // Watch how the path evolves (relay -> direct) in the background.
    tokio::spawn(report_paths(conn.clone(), "connect"));

    // Send a ping every second so there is traffic for hole punching to act on.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(args.seconds);
    let mut tick = 0u64;
    while tokio::time::Instant::now() < deadline {
        let (mut send_stream, mut recv_stream) = conn.open_bi().await.context("open bi")?;
        let msg = format!("ping {tick}");
        send_stream.write_all(msg.as_bytes()).await.context("write")?;
        send_stream.finish().context("finish")?;
        let reply = recv_stream.read_to_end(64 * 1024).await.context("read")?;
        debug_assert_eq!(reply, msg.as_bytes());
        tick += 1;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    conn.close(0u32.into(), b"done");
    ep.close().await;
    Ok(())
}
