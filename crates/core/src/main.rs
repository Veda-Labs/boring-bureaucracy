use core::run_simulation;

#[tokio::main]
async fn main() {
    if let Err(e) = run_simulation().await {
        eprintln!("Error: {:?}", e);
    }
}
