mod cli;
mod config;
mod db;
mod embedding;
mod handlers;
mod models;
mod queue;
mod vector_store;

use anyhow::{Context, Result};
use axum::{Router, routing::get};
use clap::Parser;
use lapin::{Connection, ConnectionProperties};
use qdrant_client::Qdrant;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::signal;
use tokio_retry::{Retry, strategy::ExponentialBackoff};
use tracing::{error, info};

use crate::{
    cli::{Cli, Command},
    config::Config,
    embedding::model::EmbeddingService,
};

pub struct AppContext {
    pub db_pool: sqlx::PgPool,
    pub qdrant_client: Qdrant,
    pub rabbitmq_channel: lapin::Channel,
    pub embedding_model: EmbeddingService,
    pub config: Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // TODO: extract into separate module
    // TODO: set the logger type depending on the tty mode
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Serve => run_serve().await?,
        Command::DownloadModel => {
            info!("Initializing EmbeddingService to trigger download...");
            let _ = EmbeddingService::new()?;
        }
    }

    Ok(())
}

async fn run_serve() -> Result<()> {
    let config = Config::load()?;

    info!("Connecting to PostgreSQL...");

    // TODO: maybe retry is unnecessary
    let retry_strategy = ExponentialBackoff::from_millis(100).take(5);
    let db_pool = Retry::spawn(retry_strategy.clone(), || {
        PgPoolOptions::new().connect(&config.postgres_url)
    })
    .await
    .context("Failed to connect to PostgreSQL")?;

    info!("Connecting to Qdrant...");
    let qdrant_client = Qdrant::from_url(&config.qdrant_url).build()?;
    // Retry ensure collection exists might be needed if Qdrant is starting up,
    // but the client itself is lazy/cheap. Let's retry just the network call.
    Retry::spawn(retry_strategy.clone(), || async {
        vector_store::qdrant::ensure_collection_exists(&qdrant_client, &config.qdrant_collection)
            .await
    })
    .await
    .context("Failed to connect to Qdrant or create collection")?;

    info!("Connecting to RabbitMQ...");
    let rmq_conn = Retry::spawn(retry_strategy.clone(), || {
        Connection::connect(&config.rabbitmq_url, ConnectionProperties::default())
    })
    .await
    .context("Failed to connect to RabbitMQ")?;

    let rabbitmq_channel = rmq_conn.create_channel().await?;

    info!("Initializing Embedding Model...");
    let embedding_model = EmbeddingService::new()?;

    let app_context = Arc::new(AppContext {
        db_pool,
        qdrant_client,
        rabbitmq_channel: rabbitmq_channel.clone(),
        embedding_model,
        config,
    });

    let queue_name = app_context.config.rabbitmq_consume_queue.clone();

    info!("Starting consumer...");
    let consumer_task = tokio::spawn(async move {
        if let Err(e) =
            queue::consumer::start_consumer(rabbitmq_channel, &queue_name, app_context).await
        {
            error!("Consumer error: {}", e);
        }
    });

    let health_app = Router::new().route("/healthz", get(healthz));
    // TODO: set the host and port from the config
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Starting health server on 0.0.0.0:8080");

    let health_task = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, health_app).await {
            error!("Health server error: {}", e);
        }
    });

    signal::ctrl_c().await?;
    info!("Shutting down...");

    consumer_task.abort();
    health_task.abort();

    Ok(())
}

async fn healthz() -> &'static str {
    // TODO: Add downstream checks
    "OK"
}
