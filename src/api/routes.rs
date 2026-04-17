use actix_web::{HttpResponse, Responder};
use serde_json::json;

use crate::models::prediction::PredictionRequest;
use crate::models::health::HealthResponse;
use crate::models::inference::infer_with_model;

#[actix_web::post("/predict")]
async fn predict(req: actix_web::web::Json<PredictionRequest>) -> impl Responder {
    let request = req.into_inner();
    
    match infer_with_model(&request).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": e.to_string()
        })),
    }
}

#[actix_web::get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        version: "0.1.0".to_string(),
    })
}
