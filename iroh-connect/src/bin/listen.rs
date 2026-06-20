//! Accept side of the iroh echo example.
//!
//! Binds an [`Endpoint`], prints this node's `EndpointId`, then waits for an
//! incoming QUIC connection over the `ALPN`, echoes back every message it
//! receives on a bi-directional stream.
//!
//! Run with:
//! ```bash
//! cargo run --bin listen
//! ```
//! Copy the printed Endpoint ID and pass it to the `connect` binary.

use anyhow::{Context, Result};
use iroh::{Endpoint, endpoint::presets};

/// Application-Layer Protocol Negotiation identifier shared by both peers.
const ALPN: &[u8] = b"iroh-connect/echo/0";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // To accept connections at least one ALPN must be configured.
    let ep = Endpoint::builder(presets::N0)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?;

    println!("listening as endpoint: {}", ep.id());
    println!("waiting for an incoming connection... (Ctrl-C to quit)");

    // Accept incoming connections until the endpoint is closed.
    while let Some(incoming) = ep.accept().await {
        let conn = incoming.await.context("accept conn")?;
        let remote = conn.remote_id();
        println!("accepted connection from {remote}");

        // Echo every bi-directional stream until the peer hangs up.
        loop {
            let (mut send_stream, mut recv_stream) = match conn.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => break,
            };
            let msg = recv_stream
                .read_to_end(64 * 1024)
                .await
                .context("read")?;
            println!("received {} bytes: {:?}", msg.len(), String::from_utf8_lossy(&msg));
            send_stream.write_all(&msg).await.context("write")?;
            send_stream.finish().context("finish")?;
        }
        println!("connection from {remote} closed");
    }

    ep.close().await;
    Ok(())
}
