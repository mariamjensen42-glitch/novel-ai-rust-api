use actix_web::{web, HttpResponse};

use crate::auth::CurrentUser;
use crate::db::pool::pool;
use crate::error::AppResult;
use crate::models::novel::{CreateNovelRequest, UpdateNovelRequest};
use crate::services;

pub async fn list_by_project(
    user: CurrentUser,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let project_id = path.into_inner();
    let items = services::novel_service::list_by_project(pool(), &user.id, &project_id).await?;
    Ok(HttpResponse::Ok().json(items))
}

pub async fn create(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<CreateNovelRequest>,
) -> AppResult<HttpResponse> {
    let project_id = path.into_inner();
    let n = services::novel_service::create(pool(), &user.id, &project_id, req.into_inner()).await?;
    Ok(HttpResponse::Created().json(n))
}

pub async fn get(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let n = services::novel_service::get(pool(), &user.id, &id).await?;
    Ok(HttpResponse::Ok().json(n))
}

pub async fn update(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<UpdateNovelRequest>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let n = services::novel_service::update(pool(), &user.id, &id, req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(n))
}

pub async fn delete(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    services::novel_service::delete(pool(), &user.id, &id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/projects/{project_id}/novels")
            .route(web::get().to(list_by_project))
            .route(web::post().to(create)),
    );
    cfg.service(
        web::resource("/novels/{id}")
            .route(web::get().to(get))
            .route(web::patch().to(update))
            .route(web::delete().to(delete)),
    );
}
