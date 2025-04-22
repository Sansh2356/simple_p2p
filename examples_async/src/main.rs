#![allow(unused)]
use mini_redis::{Connection, Frame};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};
#[tokio::main]
async fn main() {
    info!("logger has been initialized");
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber);

    //Appending tracing logs to file
    // let file_appender = tracing_appender::rolling::minutely("./logs/", "logger.log");
    // let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // tracing_subscriber::fmt().with_writer(non_blocking).init();

    // client_side_fun().await;
    info!(
        "Listener binded to the mini-redis server at socket address : {:?}",
        String::from("127.0.0.1:6379")
    );
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket_local_address_result = socket.local_addr().unwrap();
        info!(
            "New client has connected at IP - {:?} and PORT - {:?}",
            socket_local_address_result.ip().to_string(),socket_local_address_result.clone().port()
        );
        process(socket).await;
    }
}
#[tracing::instrument]
async fn process(socket: TcpStream) {
    let mut connection = Connection::new(socket);
    if let Some(frame) = connection.read_frame().await.unwrap() {
        info!("GOT FRAME {:?}", frame.to_string());
        let response = Frame::Error("unimplemented".to_string());
        connection.write_frame(&response).await.unwrap();
    }
}
