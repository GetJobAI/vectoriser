use crate::models::{DocumentSections, SectionType, SourceKind};

pub fn to_embed_inputs(
    sections: &DocumentSections,
    kind: SourceKind,
) -> Vec<(SectionType, String)> {
    let mut inputs = Vec::new();

    // TODO: this is ugly, could be better, maybe store the section type in the DB
    let (full_type, skills_type, exp_req_type) = match kind {
        SourceKind::Resume => (
            SectionType::ResumeFull,
            SectionType::ResumeSkills,
            SectionType::ResumeExperience,
        ),
        SourceKind::JobAnalysis => (
            SectionType::JobFull,
            SectionType::JobSkills,
            SectionType::JobRequirements,
        ),
    };

    // TODO: maybe set the threshold in the config
    if sections.full_text.len() >= 20 {
        inputs.push((full_type, sections.full_text.clone()));
    }

    if let Some(skills) = &sections.skills
        && skills.len() >= 20
    {
        inputs.push((skills_type, skills.clone()));
    }

    if let Some(exp_req) = &sections.experience_or_requirements
        && exp_req.len() >= 20
    {
        inputs.push((exp_req_type, exp_req.clone()));
    }

    if kind == SourceKind::Resume
        && let Some(edu) = &sections.education
        && edu.len() >= 20
    {
        inputs.push((SectionType::ResumeEducation, edu.clone()));
    }

    inputs
}
