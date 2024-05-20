use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use serde::Deserialize;
use tokio::sync::watch;
use warp::Filter;

use crate::davis::Davis;

#[derive(Debug, Deserialize)]
struct Args {
    duration: f64,
    interval: f64,
}

async fn query_last_data(sensor: Arc<Davis>) -> Result<impl warp::Reply, warp::Rejection> {
    match sensor.last_data().await {
        Ok(measurement) => Ok(warp::reply::json(&measurement)),
        _ => Err(warp::reject()),
    }
}

async fn query_current(sensor: Arc<Davis>) -> Result<impl warp::Reply, warp::Rejection> {
    match sensor.current_wind().await {
        Ok(measurement) => Ok(warp::reply::json(&measurement)),
        _ => Err(warp::reject()),
    }
}

async fn query_data_since(
    query: Args,
    sensor: Arc<Davis>,
) -> Result<impl warp::Reply, warp::Rejection> {
    match sensor
        .aggregated_data_since(
            Duration::from_secs_f64(query.duration),
            Duration::from_secs_f64(query.interval),
        )
        .await
    {
        Ok(measurements) => Ok(warp::reply::json(&measurements)),
        _ => Err(warp::reject()),
    }
}

pub async fn run_server(
    context: Arc<Davis>,
    addr: SocketAddr,
    mut shutdown_rx: watch::Receiver<()>,
) {
    let with_context = warp::any().map(move || context.clone());

    let current = warp::get()
        .and(warp::path("wind"))
        .and(warp::path("current"))
        .and(warp::path::end())
        .and(with_context.clone())
        .and_then(query_current);

    let last_data = warp::get()
        .and(warp::path("wind"))
        .and(warp::path("last_data"))
        .and(warp::path::end())
        .and(with_context.clone())
        .and_then(query_last_data);

    let data_since = warp::get()
        .and(warp::path("wind"))
        .and(warp::path("data_since"))
        .and(warp::query::<Args>())
        .and(with_context.clone())
        .and_then(query_data_since);

    let static_dir = warp::fs::dir("public");

    let routes = last_data
        .or(data_since)
        .or(current)
        .or(static_dir)
        .with(warp::cors().allow_any_origin());

    println!("Starting server on {:?}", &addr);
    let (_addr, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, async move {
        shutdown_rx.changed().await.ok();
        tracing::info!("closing robot rest interface");
    });
    server.await;
}

pub struct WindServer {
    handle: tokio::task::JoinHandle<()>,
    shutdown_sender: watch::Sender<()>,
}

impl WindServer {
    pub fn run(context: Arc<Davis>, addr: SocketAddr) -> Self {
        let (shutdown_sender, shutdown_rx) = watch::channel(());
        let handle =
            tokio::task::spawn(async move { run_server(context, addr, shutdown_rx).await });
        Self {
            handle,
            shutdown_sender,
        }
    }

    pub async fn stop(self) -> Result<()> {
        self.shutdown_sender.send(())?;
        Ok(self.handle.await?)
    }
}
