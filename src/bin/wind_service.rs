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
        .arg(
            arg!(--"db_path" <DATABASE> "Sets a custom database path").default_value("./db.sqlite"),
        )
        .get_matches();
    let emulation = matches.get_one::<bool>("emulation").unwrap();
    let db_path = matches.get_one::<String>("db_path").unwrap();

    println!(
        "Starting Wind Sensor Service with emulation {:?}",
        emulation
    );

    let davis = Arc::new(Davis::connect(db_path.clone(), *emulation).await?);
    let http_server_axum = WindServer::run(davis.clone(), "0.0.0.0:8080".parse()?).await?;

    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Ctrl-C catched");
        }
        Err(err) => {
            tracing::warn!("Unable to listen for shutdown signal: {err}");
        }
    }
    http_server_axum.stop().await?;

    Ok(())
}
