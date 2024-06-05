use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response, Result},
    routing::get,
    Json, Router,
};
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::watch;
use tower_http::services::ServeDir;

use crate::{davis::Davis, db::Measurement};

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

async fn current(State(davis): State<Arc<Davis>>) -> Result<Json<Measurement>, AppError> {
    Ok(Json(davis.current_wind().await?))
}

async fn oldest_data(State(davis): State<Arc<Davis>>) -> Result<Json<Measurement>, AppError> {
    Ok(Json(davis.oldest_data().await?))
}

#[derive(Debug, Deserialize)]
struct DataSinceArgs {
    duration: f64,
    interval: f64,
}

async fn data_since(
    State(davis): State<Arc<Davis>>,
    Query(params): Query<DataSinceArgs>,
) -> Result<Json<Vec<Measurement>>, AppError> {
    Ok(Json(
        davis
            .aggregated_data_since(
                Duration::from_secs_f64(params.duration),
                Duration::from_secs_f64(params.interval),
            )
            .await?,
    ))
}

pub struct WindServer {
    handle: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    shutdown_sender: watch::Sender<()>,
}

impl WindServer {
    pub async fn run(context: Arc<Davis>, addr: SocketAddr) -> anyhow::Result<Self> {
        let router = Router::new()
            .nest_service("/", ServeDir::new("public"))
            .route("/wind/current", get(current))
            .route("/wind/oldest_data", get(oldest_data))
            .route("/wind/data_since", get(data_since))
            .with_state(context);

        println!("Starting server on {:?}", &addr);
        let (shutdown_sender, mut shutdown_rx) = watch::channel(());
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let handle = tokio::task::spawn(async move {
            let res = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _res = shutdown_rx.changed().await;
                })
                .await;
            dbg!("SERVER STOPPED", &res);
            res
        });
        Ok(Self {
            handle,
            shutdown_sender,
        })
    }

    pub async fn stop(self) -> anyhow::Result<()> {
        self.shutdown_sender.send(())?;
        Ok(self.handle.await??)
    }
}
