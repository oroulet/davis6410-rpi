use anyhow::Result;

use rust_wind::api::run_server;

#[tokio::main]
async fn main() -> Result<()> {
    run_server().await;
    Ok(())
}
