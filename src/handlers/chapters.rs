use actix_web::{web, HttpResponse};

use crate::auth::CurrentUser;
use crate::db::pool::pool;
use crate::error::AppResult;
use crate::models::chapter::{CreateChapterRequest, ReorderRequest, UpdateChapterRequest};
use crate::services;

pub async fn list_by_novel(
    user: CurrentUser,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let novel_id = path.into_inner();
    let items = services::chapter_service::list_by_novel(pool(), &user.id, &novel_id).await?;
    Ok(HttpResponse::Ok().json(items))
}

pub async fn create(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<CreateChapterRequest>,
) -> AppResult<HttpResponse> {
    let novel_id = path.into_inner();
    let c = services::chapter_service::create(pool(), &user.id, &novel_id, req.into_inner()).await?;
    Ok(HttpResponse::Created().json(c))
}

pub async fn get(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let c = services::chapter_service::get(pool(), &user.id, &id).await?;
    Ok(HttpResponse::Ok().json(c))
}

pub async fn update(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<UpdateChapterRequest>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let c = services::chapter_service::update(pool(), &user.id, &id, req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(c))
}

pub async fn delete(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    services::chapter_service::delete(pool(), &user.id, &id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn reorder(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<ReorderRequest>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let c = services::chapter_service::reorder(pool(), &user.id, &id, req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(c))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/novels/{novel_id}/chapters")
            .route(web::get().to(list_by_novel))
            .route(web::post().to(create)),
    );
    cfg.service(
        web::resource("/chapters/{id}")
            .route(web::get().to(get))
            .route(web::patch().to(update))
            .route(web::delete().to(delete)),
    );
    cfg.service(
        web::resource("/chapters/{id}/reorder")
            .route(web::post().to(reorder)),
    );
}
