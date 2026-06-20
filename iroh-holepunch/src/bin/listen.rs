//! Accept side of the iroh hole-punching example.
//!
//! Binds an [`Endpoint`], prints its `EndpointId`, and for every incoming
//! connection echoes data while printing how the connection's path evolves
//! (relay -> direct) as hole punching succeeds.
//!
//! Run with:
//! ```bash
//! cargo run --bin listen
//! ```

use anyhow::{Context, Result};
use iroh::{Endpoint, endpoint::presets};
use iroh_holepunch::{ALPN, report_paths};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let ep = Endpoint::builder(presets::N0)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?;

    println!("listening as endpoint: {}", ep.id());
    println!("waiting for an incoming connection... (Ctrl-C to quit)");

    while let Some(incoming) = ep.accept().await {
        let conn = incoming.await.context("accept conn")?;
        let remote = conn.remote_id();
        println!("accepted connection from {remote}");

        // Watch how this connection's path changes over its lifetime.
        tokio::spawn(report_paths(conn.clone(), "listen"));

        // Echo every bi-directional stream until the peer hangs up. The ongoing
        // traffic is what lets the direct path get established and selected.
        loop {
            let (mut send_stream, mut recv_stream) = match conn.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => break,
            };
            let msg = recv_stream.read_to_end(64 * 1024).await.context("read")?;
            send_stream.write_all(&msg).await.context("write")?;
            send_stream.finish().context("finish")?;
        }
        println!("connection from {remote} closed");
    }

    ep.close().await;
    Ok(())
}
