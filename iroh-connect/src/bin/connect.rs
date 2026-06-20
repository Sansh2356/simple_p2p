//! Connect side of the iroh echo example.
//!
//! Binds an [`Endpoint`], dials the `EndpointId` printed by the `listen`
//! binary (resolved over the n0 discovery service), opens a bi-directional
//! stream, sends a message and prints the echoed reply.
//!
//! Run with:
//! ```bash
//! cargo run --bin connect -- <endpoint-id> [message]
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use iroh::{Endpoint, EndpointId, endpoint::presets};

/// Application-Layer Protocol Negotiation identifier shared by both peers.
const ALPN: &[u8] = b"iroh-connect/echo/0";

#[derive(Parser)]
struct Args {
    /// The `EndpointId` printed by the `listen` binary.
    endpoint_id: EndpointId,
    /// The message to send (defaults to "hello iroh").
    #[arg(default_value = "hello iroh")]
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let ep = Endpoint::bind(presets::N0).await?;
    println!("connecting to {}...", args.endpoint_id);

    // With the n0 discovery service the EndpointId alone is enough to dial.
    let conn = ep.connect(args.endpoint_id, ALPN).await?;
    let (mut send_stream, mut recv_stream) = conn.open_bi().await.context("open bi")?;

    send_stream
        .write_all(args.message.as_bytes())
        .await
        .context("write")?;
    send_stream.finish().context("finish")?;
    println!("sent: {:?}", args.message);

    let reply = recv_stream
        .read_to_end(64 * 1024)
        .await
        .context("read")?;
    println!("echoed back: {:?}", String::from_utf8_lossy(&reply));

    conn.close(0u32.into(), b"done");
    ep.close().await;
    Ok(())
}
