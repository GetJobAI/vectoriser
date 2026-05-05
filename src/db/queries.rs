use anyhow::{Result, anyhow};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::DocumentSections;

pub async fn fetch_resume(pool: &PgPool, id: Uuid) -> Result<DocumentSections> {
    let row = sqlx::query("SELECT document FROM resumes WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let row = row.ok_or_else(|| anyhow!("Resume not found for id: {}", id))?;
    let doc: serde_json::Value = row.try_get("document")?;

    // TODO: get from specific field instead of stringifying
    let full_text = doc.to_string();

    let skills = doc.get("skills").and_then(|s| s.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    });

    // TODO: use another field if the type is Job
    let experience_or_requirements = doc.get("experience").map(|e| e.to_string());

    let education = doc.get("education").map(|e| e.to_string());

    Ok(DocumentSections {
        full_text,
        skills,
        experience_or_requirements,
        education,
    })
}

pub async fn fetch_job_analysis(pool: &PgPool, id: Uuid) -> Result<DocumentSections> {
    let row = sqlx::query("SELECT document FROM job_analyses WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let row = row.ok_or_else(|| anyhow!("Job analysis not found for id: {}", id))?;
    let doc: serde_json::Value = row.try_get("document")?;

    let full_text = doc.to_string();

    let skills = doc.get("skills").map(|s| s.to_string()); // Depending on schema, can be formatted better

    let experience_or_requirements = doc.get("requirements").map(|r| r.to_string());

    Ok(DocumentSections {
        full_text,
        skills,
        experience_or_requirements,
        education: None,
    })
}
