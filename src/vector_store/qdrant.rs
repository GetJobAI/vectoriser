use anyhow::Result;
use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        Condition, CreateCollection, DeletePointsBuilder, Distance, Filter, PointStruct,
        UpsertPointsBuilder, VectorParams, VectorsConfig, vectors_config::Config,
    },
};
use serde_json::json;
use uuid::Uuid;

use crate::models::SectionType;

pub async fn ensure_collection_exists(client: &Qdrant, collection_name: &str) -> Result<()> {
    let exists = client.collection_exists(collection_name).await?;
    if !exists {
        client
            .create_collection(CreateCollection {
                collection_name: collection_name.to_string(),
                vectors_config: Some(VectorsConfig {
                    config: Some(Config::Params(VectorParams {
                        // TODO: set from config
                        size: 1024, // BGEM3 dimension
                        distance: Distance::Cosine.into(),
                        ..Default::default()
                    })),
                }),
                ..Default::default()
            })
            .await?;
    }
    Ok(())
}

pub async fn delete_vectors_for_source(
    client: &Qdrant,
    collection_name: &str,
    source_id: Uuid,
) -> Result<()> {
    client
        .delete_points(
            DeletePointsBuilder::new(collection_name).points(Filter::must([Condition::matches(
                "source_id",
                source_id.to_string(),
            )])),
        )
        .await?;
    Ok(())
}

pub async fn upsert_vectors(
    client: &Qdrant,
    collection_name: &str,
    source_id: Uuid,
    user_id: Uuid,
    embeddings: Vec<(SectionType, String, Vec<f32>)>,
) -> Result<Vec<Uuid>> {
    let mut points = Vec::with_capacity(embeddings.len());
    let mut vector_ids = Vec::with_capacity(embeddings.len());

    for (section_type, text, vector) in embeddings {
        let point_id = Uuid::new_v4();
        vector_ids.push(point_id);

        let payload = Payload::try_from(json!({
            "source_id": source_id.to_string(),
            "user_id": user_id.to_string(),
            "source_type": section_type.as_str(),
            "section_type": section_type.as_str(),
            "text": text
        }))?;

        points.push(PointStruct::new(point_id.to_string(), vector, payload));
    }

    client
        .upsert_points(UpsertPointsBuilder::new(collection_name, points))
        .await?;

    Ok(vector_ids)
}
