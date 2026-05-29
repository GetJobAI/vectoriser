use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ResumeParsedEvent {
    pub resume_id: Uuid,
    pub user_id: String,
}

impl From<ResumeParsedEvent> for DocumentParsedEvent {
    fn from(e: ResumeParsedEvent) -> Self {
        DocumentParsedEvent {
            source_id: e.resume_id,
            source_type: SourceKind::Resume,
            user_id: e.user_id,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DocumentParsedEvent {
    pub source_id: Uuid,
    pub source_type: SourceKind,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DbInsertEvent {
    pub table: String,
    pub id: Uuid,
    pub data: DbEventData,
}

#[derive(Debug, Deserialize)]
pub struct DbEventData {
    pub user_id: String,
}

impl DbInsertEvent {
    pub fn into_document_event(self) -> Option<DocumentParsedEvent> {
        let source_type = match self.table.as_str() {
            "resumes" => SourceKind::Resume,
            "job_postings" => SourceKind::JobAnalysis,
            _ => return None,
        };
        Some(DocumentParsedEvent {
            source_id: self.id,
            source_type,
            user_id: self.data.user_id,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VectorsReadyEvent {
    pub source_id: Uuid,
    pub source_type: SourceKind,
    pub vector_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Resume,
    JobAnalysis,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum SectionType {
    ResumeFull,
    ResumeSkills,
    ResumeExperience,
    ResumeEducation,
    JobFull,
    JobSkills,
    JobRequirements,
}

impl SectionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SectionType::ResumeFull => "resume_full",
            SectionType::ResumeSkills => "resume_skills",
            SectionType::ResumeExperience => "resume_experience",
            SectionType::ResumeEducation => "resume_education",
            SectionType::JobFull => "job_full",
            SectionType::JobSkills => "job_skills",
            SectionType::JobRequirements => "job_requirements",
        }
    }
}

pub struct DocumentSections {
    pub full_text: String,
    pub skills: Option<String>,
    pub experience_or_requirements: Option<String>,
    pub education: Option<String>, // resume only
}
