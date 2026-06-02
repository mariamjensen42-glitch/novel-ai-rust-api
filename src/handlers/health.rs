use actix_web::{web, HttpResponse};
use serde::Serialize;
use std::sync::Arc;

use crate::providers::registry::available_providers;

#[derive(Debug, Serialize)]
pub struct ModelHealth {
    pub name: String,
    pub status: String,
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub models: Vec<ModelHealth>,
    pub timestamp: String,
}

pub async fn health_check() -> HttpResponse {
    let providers = available_providers();
    let models: Vec<ModelHealth> = providers
        .into_iter()
        .map(|n| ModelHealth {
            name: n.to_string(),
            status: "ok".to_string(),
            error: String::new(),
        })
        .collect();
    let status = if models.is_empty() { "degraded" } else { "ok" };
    let resp = HealthResponse {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        models,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    HttpResponse::Ok().json(resp)
}

pub fn _suppress(_: Arc<reqwest::Client>) {}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health_check));
}
