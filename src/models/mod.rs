use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub title: String,
    pub content: String,
    pub start_pos: usize,
    pub end_pos: usize,
}

#[derive(Debug, Serialize)]
pub struct ProcessResult {
    pub chapters: Vec<Chapter>,
    pub epub_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LLMResponse {
    pub is_valid: bool,
    pub suggested_title: Option<String>,
    pub has_content_modified: bool,
    pub suggestions: Option<String>,
}
