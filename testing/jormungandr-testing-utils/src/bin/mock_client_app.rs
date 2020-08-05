use jormungandr_testing_utils::testing::node::grpc::client::JormungandrClient;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = args[2].parse().unwrap();
    let client = JormungandrClient::new(&args[1], port);
    let response = client.handshake().await;
    println!("{:?}", response);
}
