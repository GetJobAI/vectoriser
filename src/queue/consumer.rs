use std::sync::Arc;

use anyhow::Result;
use lapin::{
    Channel, ExchangeKind,
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    },
    types::FieldTable,
};
use tokio_stream::StreamExt;
use tracing::{error, info, instrument};

use crate::{
    AppContext, handlers,
    models::{DocumentParsedEvent, ResumeParsedEvent, SourceKind},
};

pub async fn start_consumer(
    channel: Channel,
    exchange_name: &str,
    queue_name: &str,
    routing_key: &str,
    app_context: Arc<crate::AppContext>,
) -> Result<()> {
    channel
        .exchange_declare(
            exchange_name.into(),
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    let _queue = channel
        .queue_declare(
            queue_name.into(),
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            queue_name.into(),
            exchange_name.into(),
            routing_key.into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let mut consumer = channel
        .basic_consume(
            queue_name.into(),
            "vectoriser_consumer".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!(
        exchange = exchange_name,
        queue = queue_name,
        routing_key = routing_key,
        "Started consumer"
    );

    while let Some(delivery_result) = consumer.next().await {
        match delivery_result {
            Ok(delivery) => {
                let payload = &delivery.data;
                match serde_json::from_slice::<ResumeParsedEvent>(payload) {
                    Ok(resume_event) => {
                        let event = DocumentParsedEvent::from(resume_event);
                        match handle_event(&app_context, event).await {
                            Ok(_) => {
                                let _ = delivery.ack(BasicAckOptions::default()).await;
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to process event. Nacking with requeue.");
                                let _ = delivery
                                    .nack(BasicNackOptions {
                                        requeue: true,
                                        ..Default::default()
                                    })
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to deserialize payload. Nacking without requeue.");
                        let _ = delivery
                            .nack(BasicNackOptions {
                                requeue: false,
                                ..Default::default()
                            })
                            .await;
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Consumer error.");
            }
        }
    }

    Ok(())
}

#[instrument(skip(ctx))]
// TODO: check if Arc is necessary
async fn handle_event(ctx: &Arc<AppContext>, event: DocumentParsedEvent) -> Result<()> {
    match event.source_type {
        SourceKind::Resume => handlers::resume::handle_resume_parsed(ctx, event).await,
        SourceKind::JobAnalysis => handlers::job::handle_job_parsed(ctx, event).await,
    }
}
