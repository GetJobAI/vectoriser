use anyhow::Result;
use std::sync::Arc;
use tracing::{info, instrument};

use crate::{
    AppContext,
    db::queries,
    embedding::chunker,
    models::{DocumentParsedEvent, SourceKind, VectorsReadyEvent},
    queue::publisher,
    vector_store::qdrant,
};

#[instrument(skip_all, fields(source_id = %event.source_id, source_type = "resume"))]
pub async fn handle_resume_parsed(ctx: &Arc<AppContext>, event: DocumentParsedEvent) -> Result<()> {
    let sections = queries::fetch_resume(&ctx.db_pool, event.source_id).await?;

    let embed_inputs = chunker::to_embed_inputs(&sections, SourceKind::Resume);
    if embed_inputs.is_empty() {
        info!(source_id = %event.source_id, "No sections to embed for resume");
        return Ok(());
    }

    let (section_types, texts): (Vec<_>, Vec<_>) = embed_inputs.into_iter().unzip();

    let vectors = ctx.embedding_model.embed_batch(texts).await?;

    let embeddings = section_types.into_iter().zip(vectors.into_iter()).collect();

    qdrant::delete_vectors_for_source(
        &ctx.qdrant_client,
        &ctx.config.qdrant_collection,
        event.source_id,
    )
    .await?;

    let vector_ids = qdrant::upsert_vectors(
        &ctx.qdrant_client,
        &ctx.config.qdrant_collection,
        event.source_id,
        event.user_id,
        embeddings,
    )
    .await?;

    publisher::publish_vectors_ready(
        &ctx.rabbitmq_channel,
        &ctx.config.rabbitmq_publish_exchange,
        &ctx.config.rabbitmq_publish_routing_key,
        VectorsReadyEvent {
            source_id: event.source_id,
            source_type: SourceKind::Resume,
            vector_ids,
        },
    )
    .await?;

    info!(source_id = %event.source_id, "Successfully processed resume");

    Ok(())
}
