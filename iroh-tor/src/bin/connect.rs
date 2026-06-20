//! Connect side of the iroh-over-Tor echo example.
//!
//! Dials the `EndpointId` printed by the `listen` binary. The Tor transport
//! derives the peer's `.onion` address from that `EndpointId` and routes the
//! QUIC connection through the Tor network, so connectivity needs no IP
//! address, no NAT hole-punching, and no public relay.
//!
//! Requires a running Tor daemon with the control port enabled:
//! ```bash
//! tor --ControlPort 9051 --CookieAuthentication 0
//! ```
//!
//! Run with:
//! ```bash
//! cargo run --bin connect -- <endpoint-id> [message]
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use iroh::{Endpoint, EndpointId, SecretKey};
use iroh_tor_transport::TorCustomTransport;

/// Application-Layer Protocol Negotiation identifier shared by both peers.
const ALPN: &[u8] = b"iroh-tor/echo/0";

#[derive(Parser)]
struct Args {
    /// The `EndpointId` printed by the `listen` binary.
    endpoint_id: EndpointId,
    /// The message to send (defaults to "hello over tor").
    #[arg(default_value = "hello over tor")]
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // The dialer needs its own hidden service too: Tor circuits are one-way, so
    // each side publishes an onion address to receive the other's streams.
    let secret = SecretKey::generate();
    let transport = TorCustomTransport::builder()
        .build(secret.clone())
        .await
        .context("failed to create Tor transport — is Tor running with ControlPort 9051?")?;

    let ep = Endpoint::builder(transport.preset())
        .secret_key(secret)
        .bind()
        .await?;

    println!("connecting to {} over Tor (this can take a while)...", args.endpoint_id);

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
