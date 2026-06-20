//! Shared helpers for the iroh hole-punching example.
//!
//! iroh hole-punches automatically: a connection usually starts out routed
//! through a relay server (which also coordinates the hole punch), and once the
//! two peers have discovered each other's external addresses they upgrade to a
//! *direct* path. This module just *observes* that upgrade so it can be printed.

use futures_lite::StreamExt;
use iroh::endpoint::Connection;

/// Application-Layer Protocol Negotiation identifier shared by both peers.
pub const ALPN: &[u8] = b"iroh-holepunch/echo/0";

/// Watch a connection's network paths and print each change to the *selected*
/// path, so you can see it move from `RELAY` to `DIRECT` once hole punching
/// succeeds. Returns when the connection closes.
pub async fn report_paths(conn: Connection, label: &str) {
    let mut stream = conn.paths_stream();
    let mut last = String::new();
    while let Some(paths) = stream.next().await {
        // Summarise the currently selected (actively used) path.
        let summary = paths
            .iter()
            .find(|p| p.is_selected())
            .map(|p| {
                let kind = if p.is_relay() {
                    "RELAY  (routed via relay server)"
                } else {
                    "DIRECT (hole-punched)"
                };
                format!(
                    "{kind} -> {} (rtt {:?}, {} path(s) open)",
                    p.remote_addr(),
                    p.rtt(),
                    paths.len()
                )
            })
            .unwrap_or_else(|| "no path selected yet".to_string());

        if summary != last {
            println!("[{label}] {summary}");
            last = summary;
        }
    }
    println!("[{label}] connection closed");
}
