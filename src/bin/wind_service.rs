use std::sync::Arc;

use anyhow::Result;

use clap::{arg, Command};
use rust_wind::{api::WindServer, davis::Davis};
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

    let davis = Arc::new(Davis::connect(String::from("./db.sqlite"), *emulation).await?);
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
