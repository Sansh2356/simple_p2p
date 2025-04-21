#![allow(unused)]
use mini_redis::{Connection, Frame};
use tokio::net::{TcpListener, TcpStream};
#[tokio::main]
async fn main() {
    // client_side_fun().await;
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        process(socket).await;
    }
}

async fn process(socket: TcpStream) {
    let mut connection = Connection::new(socket);
    if let Some(frame) = connection.read_frame().await.unwrap() {
        println!("GOT: {:?}", frame);

        // let response = Frame::Error("unimplemented".to_string());
        // connection.write_frame(&response).await.unwrap();
    }
}
