use anyhow::Result;

use rust_wind::{api::run_server, davis::Davis};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    //run_server().await;
    let _davis = Davis::connect().await;

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
