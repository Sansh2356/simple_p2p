pub mod client;
use client::client_side::client_side_fun;
#[tokio::main]

async fn main() {
    loop {
        let result = client_side_fun().await;
        match result {
            Ok(_) => {}
            Err(error) => {
                println!(
                    "An error occurred while setting up dummy client {:?}",
                    error
                );
            }
        }
    }
}
