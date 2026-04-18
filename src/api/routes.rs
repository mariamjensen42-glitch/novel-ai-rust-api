use actix_web::{HttpResponse, Responder, web};
use tracing::{info, debug, warn};
use reqwest::Client;
use serde_json::json;
use std::time::{SystemTime, Duration};
use std::sync::Arc;

use crate::models::prediction::PredictionRequest;
use crate::models::health::{HealthResponse, ModelHealth};
use crate::models::error::AppError;
use crate::models::inference::infer_with_model;
use crate::models::cache::{SharedCache, generate_cache_key};
use crate::config::get_config;

/// 预测API端点
#[actix_web::post("/predict")]
pub async fn predict(
    req: actix_web::web::Json<PredictionRequest>,
    cache: web::Data<SharedCache>,
    http_client: web::Data<Arc<Client>>
) -> Result<impl Responder, AppError> {
    let request = req.into_inner();
    
    // 验证请求参数
    if let Err(error_message) = request.validate() {
        return Err(AppError::bad_request(&error_message));
    }
    
    // 生成缓存键
    let cache_key = generate_cache_key(&request);
    
    // 尝试从缓存中获取响应
    if let Ok(mut cache_lock) = cache.lock() {
        if let Some(cached_response) = cache_lock.get(&cache_key) {
            info!("Cache hit for prediction request: model={}", request.model);
            return Ok(HttpResponse::Ok().json(cached_response));
        }
    }
    
    info!("Cache miss for prediction request: model={}, prompt_length={}, max_tokens={:?}, temperature={:?}", 
          request.model, request.prompt.len(), request.max_tokens, request.temperature);
    
    // 执行预测
    let response = infer_with_model(&request, &http_client).await?;
    
    // 将结果存入缓存
    if let Ok(mut cache_lock) = cache.lock() {
        cache_lock.set(cache_key, response.clone());
        debug!("Prediction response cached successfully");
    }
    
    info!("Prediction completed successfully: model={}, tokens_used={}", response.model, response.tokens_used);
    Ok(HttpResponse::Ok().json(response))
}

/// 检查单个模型的健康状态
async fn check_model_health(model_name: &str, endpoint: &str, api_key: &str, client: &Client) -> ModelHealth {
    info!("Checking health for model: {}", model_name);
    
    let timeout = Duration::from_secs(5);
    
    // 发送一个简单的健康检查请求
    let response = client.post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "prompt": "health check",
            "max_tokens": 1,
            "temperature": 0.7,
        }))
        .timeout(timeout)
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                ModelHealth {
                    name: model_name.to_string(),
                    status: "ok".to_string(),
                    error: "".to_string(),
                }
            } else {
                ModelHealth {
                    name: model_name.to_string(),
                    status: "error".to_string(),
                    error: format!("HTTP error: {}", resp.status()),
                }
            }
        },
        Err(e) => {
            warn!("Model health check failed for {}: error={}", model_name, e);
            ModelHealth {
                name: model_name.to_string(),
                status: "error".to_string(),
                error: e.to_string(),
            }
        },
    }
}

/// 健康检查API端点
#[actix_web::get("/health")]
pub async fn health_check(http_client: web::Data<Arc<Client>>) -> impl Responder {
    info!("Health check requested");
    
    let config = get_config();
    
    // 检查模型服务健康状态
    let deepseek_health = check_model_health("deepseek", &config.deepseek_endpoint, &config.deepseek_api_key, &http_client).await;
    let qwen_health = check_model_health("qwen", &config.qwen_endpoint, &config.qwen_api_key, &http_client).await;
    
    // 确定整体服务状态
    let all_models_healthy = deepseek_health.status == "ok" && qwen_health.status == "ok";
    let overall_status = if all_models_healthy { "ok" } else { "degraded" };
    
    // 获取当前系统时间
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    
    HttpResponse::Ok().json(HealthResponse {
        status: overall_status.to_string(),
        version: "0.1.0".to_string(),
        models: vec![deepseek_health, qwen_health],
        timestamp,
    })
}
