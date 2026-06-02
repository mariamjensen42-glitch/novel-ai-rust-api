use actix_web::{web, HttpResponse};

use crate::auth::CurrentUser;
use crate::db::pool::pool;
use crate::error::AppResult;
use crate::models::project::{CreateProjectRequest, UpdateProjectRequest};
use crate::services;

pub async fn list(user: CurrentUser) -> AppResult<HttpResponse> {
    let items = services::project_service::list(pool(), &user.id).await?;
    Ok(HttpResponse::Ok().json(items))
}

pub async fn create(
    user: CurrentUser,
    req: web::Json<CreateProjectRequest>,
) -> AppResult<HttpResponse> {
    let p = services::project_service::create(pool(), &user.id, req.into_inner()).await?;
    Ok(HttpResponse::Created().json(p))
}

pub async fn get(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let p = services::project_service::get(pool(), &user.id, &id).await?;
    Ok(HttpResponse::Ok().json(p))
}

pub async fn update(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<UpdateProjectRequest>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let p = services::project_service::update(pool(), &user.id, &id, req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(p))
}

pub async fn delete(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    services::project_service::delete(pool(), &user.id, &id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/projects")
            .route(web::get().to(list))
            .route(web::post().to(create)),
    );
    cfg.service(
        web::resource("/projects/{id}")
            .route(web::get().to(get))
            .route(web::patch().to(update))
            .route(web::delete().to(delete)),
    );
}
