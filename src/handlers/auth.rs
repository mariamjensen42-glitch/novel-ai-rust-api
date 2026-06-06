use actix_web::{web, HttpResponse};

use crate::auth::CurrentUser;
use crate::db::pool::pool;
use crate::error::AppResult;
use crate::models::user::{LoginRequest, RegisterRequest};
use crate::services;

pub async fn register(req: web::Json<RegisterRequest>) -> AppResult<HttpResponse> {
    let resp = services::auth_service::register(pool(), req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(resp))
}

pub async fn login(req: web::Json<LoginRequest>) -> AppResult<HttpResponse> {
    let resp = services::auth_service::login(pool(), req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(resp))
}

pub async fn me(user: CurrentUser) -> AppResult<HttpResponse> {
    let me = services::auth_service::me(pool(), &user.id).await?;
    Ok(HttpResponse::Ok().json(me))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/me", web::get().to(me)),
    );
}
