use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct EmbeddingService {
    // TODO: check if this is necessary
    model: Arc<Mutex<TextEmbedding>>,
}

impl EmbeddingService {
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::BGEM3))?;

        Ok(Self {
            model: Arc::new(Mutex::new(model)),
        })
    }

    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        // Since embed is a blocking operation and we're in async, we should ideally use spawn_blocking
        // but fastembed might have thread safety built-in.
        // We'll wrap in spawn_blocking for safety.
        let model = self.model.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut model_lock = model.blocking_lock();
            model_lock.embed(texts, None)
        })
        .await??;

        Ok(result)
    }
}
