#![allow(unused)]
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use tracing::info;
use tracing_attributes::instrument;

use std::{error::Error, io, net::SocketAddr};
pub mod dummy;
#[instrument]
async fn connect(addr: &SocketAddr) -> io::Result<TcpStream> {
    let stream = TcpStream::connect(&addr).await;
    tracing::info!("created stream");
    stream
}

#[instrument]
async fn write(stream: &mut TcpStream) -> io::Result<usize> {
    let result = stream.write(b"hello world\n").await;
    info!("wrote to stream; success={:?}", result.is_ok());
    result
}

// #[tokio::main]
fn main() {

    dummy::utils::dummy_test();
}
