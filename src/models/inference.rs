use reqwest::Client;
use serde_json::json;

use crate::models::prediction::{PredictionRequest, PredictionResponse};
use crate::config::get_config;

pub async fn infer_with_model(request: &PredictionRequest) -> Result<PredictionResponse, Box<dyn std::error::Error>> {
    let config = get_config();
    let client = Client::new();
    
    match request.model.as_str() {
        "deepseek" => {
            let response = client.post(&config.deepseek_endpoint)
                .header("Authorization", format!("Bearer {}", config.deepseek_api_key))
                .json(&json!({
                    "prompt": request.prompt,
                    "max_tokens": request.max_tokens.unwrap_or(100),
                    "temperature": request.temperature.unwrap_or(0.7),
                }))
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?;
            
            Ok(PredictionResponse {
                model: "deepseek".to_string(),
                generated_text: response["generated_text"].as_str().unwrap_or("").to_string(),
                tokens_used: response["tokens_used"].as_u64().unwrap_or(0) as u32,
            })
        },
        "qwen" => {
            let response = client.post(&config.qwen_endpoint)
                .header("Authorization", format!("Bearer {}", config.qwen_api_key))
                .json(&json!({
                    "prompt": request.prompt,
                    "max_tokens": request.max_tokens.unwrap_or(100),
                    "temperature": request.temperature.unwrap_or(0.7),
                }))
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?;
            
            Ok(PredictionResponse {
                model: "qwen".to_string(),
                generated_text: response["generated_text"].as_str().unwrap_or("").to_string(),
                tokens_used: response["tokens_used"].as_u64().unwrap_or(0) as u32,
            })
        },
        _ => Err("Unsupported model".into()),
    }
}
