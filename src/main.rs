#[tokio::main]
async fn main() {
    client_decision_receipt::run().await.expect("server failed");
}
