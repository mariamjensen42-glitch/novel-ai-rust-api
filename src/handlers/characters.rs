use actix_web::{web, HttpResponse};

use crate::auth::CurrentUser;
use crate::db::pool::pool;
use crate::error::AppResult;
use crate::models::character::{CreateCharacterRequest, UpdateCharacterRequest};
use crate::services;

pub async fn list_by_novel(
    user: CurrentUser,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let novel_id = path.into_inner();
    let items = services::character_service::list_by_novel(pool(), &user.id, &novel_id).await?;
    Ok(HttpResponse::Ok().json(items))
}

pub async fn create(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<CreateCharacterRequest>,
) -> AppResult<HttpResponse> {
    let novel_id = path.into_inner();
    let c = services::character_service::create(pool(), &user.id, &novel_id, req.into_inner()).await?;
    Ok(HttpResponse::Created().json(c))
}

pub async fn get(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let c = services::character_service::get(pool(), &user.id, &id).await?;
    Ok(HttpResponse::Ok().json(c))
}

pub async fn update(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<UpdateCharacterRequest>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let c = services::character_service::update(pool(), &user.id, &id, req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(c))
}

pub async fn delete(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    services::character_service::delete(pool(), &user.id, &id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/novels/{novel_id}/characters")
            .route(web::get().to(list_by_novel))
            .route(web::post().to(create)),
    );
    cfg.service(
        web::resource("/characters/{id}")
            .route(web::get().to(get))
            .route(web::patch().to(update))
            .route(web::delete().to(delete)),
    );
}
