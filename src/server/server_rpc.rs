// This is the type that implements the generated World trait. It is the business logic
// and is used to start the server.
use crate::client::client_rpc::World;
use tarpc::context;
#[derive(Clone)]
pub struct HelloServer;

impl World for HelloServer {
    async fn hello(self, _: context::Context, name: String) -> String {
        format!("Hello, {name}!")
    }
}
