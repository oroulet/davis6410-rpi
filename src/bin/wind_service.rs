use std::sync::Arc;

use anyhow::Result;

use rust_wind::{api::WindServer, davis::Davis};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let davis = Arc::new(Davis::connect().await?);
    let http_server = WindServer::run(davis.clone(), "0.0.0.0:8080".parse()?).await;

    match signal::ctrl_c().await {
        Ok(()) => {
            println!("Ctrl-C catched");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {err}");
            // we also shut d(own in case of error
        }
    }
    http_server.stop().await?;

    Ok(())
}
