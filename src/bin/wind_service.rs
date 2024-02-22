use std::sync::Arc;

use anyhow::Result;

use rust_wind::{api::WindServer, davis::Davis};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    //run_server().await;
    let davis = Arc::new(Davis::connect().await?);
    WindServer::run(davis.clone(), "127.0.0.1:80".parse()?).await;

    match signal::ctrl_c().await {
        Ok(()) => {
            println!("Ctrl-C catched");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {err}");
            // we also shut d(own in case of error
        }
    }

    Ok(())
}
