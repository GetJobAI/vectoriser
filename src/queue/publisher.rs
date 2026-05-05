use anyhow::Result;
use lapin::{BasicProperties, Channel, options::BasicPublishOptions};

use crate::models::VectorsReadyEvent;

pub async fn publish_vectors_ready(
    channel: &Channel,
    exchange: &str,
    routing_key: &str,
    event: VectorsReadyEvent,
) -> Result<()> {
    let payload = serde_json::to_vec(&event)?;

    channel
        .basic_publish(
            exchange.into(),
            routing_key.into(),
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default()
                .with_content_type("application/json".into())
                .with_delivery_mode(2), // persistent
        )
        .await?;

    Ok(())
}
