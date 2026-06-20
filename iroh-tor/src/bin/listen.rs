//! Accept side of the iroh-over-Tor echo example.
//!
//! Builds an [`Endpoint`] whose only transport is a Tor hidden service
//! (`iroh-tor-transport`). The hidden service is created automatically and its
//! `.onion` address is derived from the node's `EndpointId`, so peers can reach
//! this node knowing *only* its `EndpointId` — no IP address, no NAT traversal,
//! and no public relay required.
//!
//! Requires a running Tor daemon with the control port enabled:
//! ```bash
//! tor --ControlPort 9051 --CookieAuthentication 0
//! ```
//!
//! Run with:
//! ```bash
//! cargo run --bin listen
//! ```
//! Reuse the same identity (and therefore the same onion address) across runs
//! by exporting the printed `IROH_SECRET` before starting.

use anyhow::{Context, Result};
use data_encoding::HEXLOWER;
use iroh::{Endpoint, SecretKey};
use iroh_tor_transport::TorCustomTransport;

/// Application-Layer Protocol Negotiation identifier shared by both peers.
const ALPN: &[u8] = b"iroh-tor/echo/0";

/// Load a stable secret key from `IROH_SECRET` (hex), or generate a fresh one
/// and print it so the operator can pin the identity for future runs.
fn load_secret() -> Result<SecretKey> {
    match std::env::var("IROH_SECRET") {
        Ok(hex) => {
            let bytes = HEXLOWER
                .decode(hex.as_bytes())
                .context("invalid IROH_SECRET (expected 32-byte hex)")?;
            let bytes: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .context("invalid IROH_SECRET length (expected 32 bytes)")?;
            Ok(SecretKey::from_bytes(&bytes))
        }
        Err(_) => {
            let key = SecretKey::generate();
            println!(
                "generated a new identity; reuse it with:\n  export IROH_SECRET={}",
                HEXLOWER.encode(&key.to_bytes())
            );
            Ok(key)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let secret = load_secret()?;

    // Building the transport connects to Tor's control port and publishes an
    // ephemeral hidden service for this identity.
    let transport = TorCustomTransport::builder()
        .build(secret.clone())
        .await
        .context("failed to create Tor transport — is Tor running with ControlPort 9051?")?;

    // `preset()` wires up both the Tor transport and the discovery service that
    // maps an `EndpointId` to its derived `.onion` address.
    let ep = Endpoint::builder(transport.preset())
        .secret_key(secret)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?;

    println!("listening over Tor as endpoint: {}", ep.id());
    println!("waiting for an incoming connection... (Ctrl-C to quit)");

    while let Some(incoming) = ep.accept().await {
        let conn = incoming.await.context("accept conn")?;
        let remote = conn.remote_id();
        println!("accepted connection from {remote}");

        loop {
            let (mut send_stream, mut recv_stream) = match conn.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => break,
            };
            let msg = recv_stream
                .read_to_end(64 * 1024)
                .await
                .context("read")?;
            println!(
                "received {} bytes: {:?}",
                msg.len(),
                String::from_utf8_lossy(&msg)
            );
            send_stream.write_all(&msg).await.context("write")?;
            send_stream.finish().context("finish")?;
        }
        println!("connection from {remote} closed");
    }

    ep.close().await;
    Ok(())
}
