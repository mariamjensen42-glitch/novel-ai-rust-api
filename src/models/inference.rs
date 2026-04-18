use reqwest::Client;
use serde_json::json;
use tracing::{info, warn};
use std::sync::Arc;

use crate::models::prediction::{PredictionRequest, PredictionResponse};
use crate::models::error::AppError;
use crate::config::get_config;

pub async fn infer_with_model(request: &PredictionRequest, http_client: &Arc<Client>) -> Result<PredictionResponse, AppError> {
    let config = get_config();
    
    info!("Starting model inference: model={}, prompt_length={}", request.model, request.prompt.len());
    
    match request.model.as_str() {
        "deepseek" => {
            info!("Calling DeepSeek API: endpoint={}", config.deepseek_endpoint);
            let response = http_client.post(&config.deepseek_endpoint)
                .header("Authorization", format!("Bearer {}", config.deepseek_api_key))
                .json(&json!({
                    "prompt": request.prompt,
                    "max_tokens": request.max_tokens.unwrap_or(100),
                    "temperature": request.temperature.unwrap_or(0.7),
                }))
                .send()
                .await
                .map_err(|e| {
                    warn!("DeepSeek API request failed: error={}", e);
                    AppError::service_unavailable(&format!("API request failed: {}", e))
                })?
                .json::<serde_json::Value>()
                .await
                .map_err(|e| {
                    warn!("Failed to parse DeepSeek response: error={}", e);
                    AppError::internal_server_error(&format!("Failed to parse response: {}", e))
                })?;
            
            let response = PredictionResponse {
                model: "deepseek".to_string(),
                generated_text: response["generated_text"].as_str().unwrap_or("").to_string(),
                tokens_used: response["tokens_used"].as_u64().unwrap_or(0) as u32,
            };
            
            info!("DeepSeek inference completed: tokens_used={}", response.tokens_used);
            Ok(response)
        },
        "qwen" => {
            info!("Calling Qwen API: endpoint={}", config.qwen_endpoint);
            let response = http_client.post(&config.qwen_endpoint)
                .header("Authorization", format!("Bearer {}", config.qwen_api_key))
                .json(&json!({
                    "prompt": request.prompt,
                    "max_tokens": request.max_tokens.unwrap_or(100),
                    "temperature": request.temperature.unwrap_or(0.7),
                }))
                .send()
                .await
                .map_err(|e| {
                    warn!("Qwen API request failed: error={}", e);
                    AppError::service_unavailable(&format!("API request failed: {}", e))
                })?
                .json::<serde_json::Value>()
                .await
                .map_err(|e| {
                    warn!("Failed to parse Qwen response: error={}", e);
                    AppError::internal_server_error(&format!("Failed to parse response: {}", e))
                })?;
            
            let response = PredictionResponse {
                model: "qwen".to_string(),
                generated_text: response["generated_text"].as_str().unwrap_or("").to_string(),
                tokens_used: response["tokens_used"].as_u64().unwrap_or(0) as u32,
            };
            
            info!("Qwen inference completed: tokens_used={}", response.tokens_used);
            Ok(response)
        },
        _ => {
            warn!("Unsupported model requested: model={}", request.model);
            Err(AppError::bad_request("Unsupported model"))
        },
    }
}
