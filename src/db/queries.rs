use anyhow::{Result, anyhow};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::DocumentSections;

fn extract_all_text(value: &serde_json::Value) -> String {
    let mut texts = Vec::new();
    match value {
        serde_json::Value::String(s) => texts.push(s.clone()),
        serde_json::Value::Array(arr) => {
            for item in arr {
                let t = extract_all_text(item);
                if !t.is_empty() {
                    texts.push(t);
                }
            }
        }
        serde_json::Value::Object(obj) => {
            for v in obj.values() {
                let t = extract_all_text(v);
                if !t.is_empty() {
                    texts.push(t);
                }
            }
        }
        _ => {}
    }
    texts.join("\n")
}

pub async fn fetch_resume(pool: &PgPool, id: Uuid) -> Result<DocumentSections> {
    let row = sqlx::query("SELECT content FROM resumes WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let row = row.ok_or_else(|| anyhow!("Resume not found for id: {}", id))?;
    let doc: serde_json::Value = row.try_get("content")?;

    let full_text = extract_all_text(&doc);

    let skills = doc
        .get("skills")
        .map(extract_all_text)
        .filter(|s| !s.is_empty());
    let experience_or_requirements = doc
        .get("experience")
        .map(extract_all_text)
        .filter(|s| !s.is_empty());
    let education = doc
        .get("education")
        .map(extract_all_text)
        .filter(|s| !s.is_empty());

    Ok(DocumentSections {
        full_text,
        skills,
        experience_or_requirements,
        education,
    })
}

pub async fn fetch_job_analysis(pool: &PgPool, id: Uuid) -> Result<DocumentSections> {
    let row = sqlx::query("SELECT content FROM job_postings WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let row = row.ok_or_else(|| anyhow!("Job analysis not found for id: {}", id))?;
    let doc: serde_json::Value = row.try_get("content")?;

    let full_text = extract_all_text(&doc);

    let skills = doc
        .get("skills")
        .map(extract_all_text)
        .filter(|s| !s.is_empty());
    let experience_or_requirements = doc
        .get("requirements")
        .or_else(|| doc.get("experience"))
        .map(extract_all_text)
        .filter(|s| !s.is_empty());

    Ok(DocumentSections {
        full_text,
        skills,
        experience_or_requirements,
        education: None,
    })
}
