use crate::models::{Chapter, LLMResponse};
use anyhow::Result;
use reqwest;
use serde_json::json;

pub struct LLMClient {
    client: reqwest::Client,
    api_url: String,
    api_key: String,
}

impl LLMClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("LLM_API_KEY").unwrap_or_else(|_| "dummy_key".to_string()); // In production, make this required
        let api_url = std::env::var("LLM_API_URL")
            .unwrap_or_else(|_| "http://localhost:11434/api/generate".to_string()); // Using Ollama as default

        Ok(LLMClient {
            client: reqwest::Client::new(),
            api_url,
            api_key,
        })
    }

    pub async fn validate_chapter(&self, chapter: &Chapter) -> Result<LLMResponse> {
        let prompt = format!(
            "Analyze this text segment in any language (including Chinese) and determine if it represents a complete chapter in a book.\n\nContent: {}\n\nRespond with JSON: {{\"is_valid\": boolean, \"suggested_title\": string or null, \"has_content_modified\": false, \"suggestions\": string or null}}",
            chapter.content
        );

        let mut request_builder = self
            .client
            .post(&self.api_url)
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": "llama2", // Default model, can be configured
                "prompt": prompt,
                "stream": false,
                "options": {
                    "temperature": 0.1
                }
            }));

        // Add authorization header if API key is provided and not dummy
        if self.api_key != "dummy_key" {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request_builder.send().await?;

        let response_text = response.text().await?;

        // Parse the response - the LLM response might be in a different format
        // depending on the API used (Ollama, OpenAI, etc.)
        let llm_response: LLMResponse =
            serde_json::from_str(&response_text).unwrap_or(LLMResponse {
                is_valid: true,
                suggested_title: None,
                has_content_modified: false,
                suggestions: None,
            });

        Ok(llm_response)
    }

    pub async fn compare_adjacent_chapters(
        &self,
        chapter1: &Chapter,
        chapter2: &Chapter,
    ) -> Result<LLMResponse> {
        let prompt = format!(
            "You are reviewing the boundary between two consecutive text segments in any language (including Chinese) that were automatically segmented as chapters. Determine if the segmentation is appropriate.\n\nFirst segment: {}\n\nSecond segment: {}\n\nRespond with JSON: {{\"is_valid\": boolean, \"suggested_title\": string or null, \"has_content_modified\": false, \"suggestions\": string or null}}",
            chapter1.content, chapter2.content
        );

        let mut request_builder = self
            .client
            .post(&self.api_url)
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": "llama2",
                "prompt": prompt,
                "stream": false,
                "options": {
                    "temperature": 0.1
                }
            }));

        // Add authorization header if API key is provided and not dummy
        if self.api_key != "dummy_key" {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request_builder.send().await?;

        let response_text = response.text().await?;

        let llm_response: LLMResponse =
            serde_json::from_str(&response_text).unwrap_or(LLMResponse {
                is_valid: true,
                suggested_title: None,
                has_content_modified: false,
                suggestions: None,
            });

        Ok(llm_response)
    }
}
