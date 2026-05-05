use std::sync::Arc;

use anyhow::Result;
use lapin::{
    Channel,
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, QueueDeclareOptions},
    types::FieldTable,
};
use tokio_stream::StreamExt;
use tracing::{error, info, instrument};

use crate::{
    AppContext, handlers,
    models::{DocumentParsedEvent, SourceKind},
};

pub async fn start_consumer(
    channel: Channel,
    queue_name: &str,
    app_context: Arc<crate::AppContext>,
) -> Result<()> {
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

    let mut consumer = channel
        .basic_consume(
            queue_name.into(),
            "vectoriser_consumer".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!(queue = queue_name, "Started consumer");

    while let Some(delivery_result) = consumer.next().await {
        match delivery_result {
            Ok(delivery) => {
                let payload = &delivery.data;
                match serde_json::from_slice::<DocumentParsedEvent>(payload) {
                    Ok(event) => match handle_event(&app_context, event).await {
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
                    },
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
