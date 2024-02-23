use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use tokio::sync::watch;
use warp::Filter;

use crate::davis::Davis;

async fn state_query(sensor: Arc<Davis>) -> Result<impl warp::Reply, warp::Rejection> {
    let speed = sensor.get_current_wind();
    Ok(warp::reply::html(format!("Current speed is {}", speed)))
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
        .and(warp::path("v1"))
        .and(warp::path("current_wind"))
        .and(warp::path::end())
        .and(with_context.clone())
        .and_then(state_query);

    let routes = root
        .or(hello)
        .or(live)
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
