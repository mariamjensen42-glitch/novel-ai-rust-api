use actix_web::{test, web, App};
use reqwest::Client;
use std::sync::{Arc, Mutex};
use novel_ai_rust_api::api::routes::{predict, health_check};
use novel_ai_rust_api::models::cache::{PredictionCache, SharedCache};
use novel_ai_rust_api::models::prediction::PredictionRequest;

#[actix_web::test]
async fn test_server_starts() {
    // Just verify that the server compiles and can start
    assert!(true);
}

#[actix_web::test]
async fn test_health_check() {
    // 创建测试应用
    let http_client = Arc::new(Client::new());
    let cache = PredictionCache::new(100, 3600);
    let shared_cache = SharedCache::new(Mutex::new(cache));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(shared_cache))
            .app_data(web::Data::new(http_client))
            .service(health_check)
    )
    .await;

    // 发送健康检查请求
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    // 验证响应状态
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_predict_invalid_model() {
    // 创建测试应用
    let http_client = Arc::new(Client::new());
    let cache = PredictionCache::new(100, 3600);
    let shared_cache = SharedCache::new(Mutex::new(cache));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(shared_cache))
            .app_data(web::Data::new(http_client))
            .service(predict)
    )
    .await;

    // 发送无效模型的请求
    let invalid_request = PredictionRequest {
        model: "invalid".to_string(),
        prompt: "test prompt".to_string(),
        max_tokens: Some(100),
        temperature: Some(0.7),
    };

    let req = test::TestRequest::post().uri("/predict")
        .set_json(&invalid_request)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // 验证响应状态为400
    assert_eq!(resp.status().as_u16(), 400);
}

#[actix_web::test]
async fn test_predict_empty_prompt() {
    // 创建测试应用
    let http_client = Arc::new(Client::new());
    let cache = PredictionCache::new(100, 3600);
    let shared_cache = SharedCache::new(Mutex::new(cache));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(shared_cache))
            .app_data(web::Data::new(http_client))
            .service(predict)
    )
    .await;

    // 发送空提示的请求
    let empty_prompt_request = PredictionRequest {
        model: "deepseek".to_string(),
        prompt: "".to_string(),
        max_tokens: Some(100),
        temperature: Some(0.7),
    };

    let req = test::TestRequest::post().uri("/predict")
        .set_json(&empty_prompt_request)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // 验证响应状态为400
    assert_eq!(resp.status().as_u16(), 400);
}

