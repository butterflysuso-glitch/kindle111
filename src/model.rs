use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub title: String,
    pub body: String,
    pub source_id: String,
    pub source_url: String,
    #[serde(default)]
    pub ocr: bool,
    #[serde(default)]
    pub ocr_evidence: Option<OcrEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrEvidence {
    pub image_sha256: String,
    pub review_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub source_id: String,
    #[serde(default)]
    pub source_url: String,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub title: String,
    pub author: String,
    pub chapter_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub kind: String,
    pub chapter: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub schema: String,
    pub title: String,
    pub author: String,
    pub chapter_count: usize,
    pub issues: Vec<Issue>,
    pub chapter_hashes: Vec<String>,
    pub completion_verified: bool,
    pub publication_permitted: bool,
    pub review_status: String,
}
