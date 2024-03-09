use std::sync::Arc;

use anyhow::Result;

use clap::{arg, Command};
use davis_rpi::{api::WindServer, davis::Davis};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let matches = Command::new("Wind sensor service")
        .author("Olivier")
        .version("0.1.0")
        .about("start wind sensor service")
        .arg(arg!(--"emulation"))
        .get_matches();
    let emulation = matches.get_one::<bool>("emulation").unwrap();

    println!(
        "Starting Wind Sensor Service with emulation {:?}",
        emulation
    );

    let davis = Arc::new(Davis::connect(String::from("./db.sqlite"), *emulation).await?);
    let http_server = WindServer::run(davis.clone(), "0.0.0.0:8080".parse()?).await;

    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Ctrl-C catched");
        }
        Err(err) => {
            tracing::warn!("Unable to listen for shutdown signal: {err}");
        }
    }
    http_server.stop().await?;

    Ok(())
}
