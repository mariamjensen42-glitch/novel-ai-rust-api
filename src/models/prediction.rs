use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PredictionRequest {
    pub model: String, // "deepseek" or "qwen"
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PredictionResponse {
    pub model: String,
    pub generated_text: String,
    pub tokens_used: u32,
}
