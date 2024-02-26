use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use serde::Deserialize;
use tokio::sync::watch;
use warp::Filter;

use crate::davis::Davis;

#[derive(Debug, Deserialize)]
struct MyDuration {
    duration: f64,
}

async fn state_query(sensor: Arc<Davis>) -> Result<impl warp::Reply, warp::Rejection> {
    let speed = sensor.last_data().await;
    Ok(warp::reply::html(format!("Current speed is {:?}", speed)))
}

async fn handle_request(
    query: MyDuration,
    sensor: Arc<Davis>,
) -> Result<impl warp::Reply, warp::Rejection> {
    match sensor
        .data_since(Duration::from_secs_f64(query.duration))
        .await
    {
        Ok(measurements) => Ok(warp::reply::html(format!("{:?}", measurements))),
        _ => Err(warp::reject()),
    }
}

pub async fn run_server(
    context: Arc<Davis>,
    addr: SocketAddr,
    mut shutdown_rx: watch::Receiver<()>,
) {
    let with_context = warp::any().map(move || context.clone());
    let root = warp::path::end().map(|| " I am a robot in state ");
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    let live = warp::get()
        .and(warp::path("wind"))
        .and(warp::path("last_data"))
        .and(warp::path::end())
        .and(with_context.clone())
        .and_then(state_query);

    let data_since = warp::get()
        .and(warp::path("wind"))
        .and(warp::path("data_since"))
        .and(warp::query::<MyDuration>())
        .and(with_context.clone())
        .and_then(handle_request);

    let routes = root
        .or(hello)
        .or(live)
        .or(data_since)
        .with(warp::cors().allow_any_origin());

    tracing::warn!("Starting server on {:?}", &addr);
    let (_addr, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, async move {
        shutdown_rx.changed().await.ok();
        tracing::warn!("closing robot rest interface");
    });
    server.await
}

pub struct WindServer {
    handle: tokio::task::JoinHandle<()>,
    shutdown_sender: watch::Sender<()>,
}

impl WindServer {
    pub async fn run(context: Arc<Davis>, addr: SocketAddr) -> Self {
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
