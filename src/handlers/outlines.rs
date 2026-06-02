use actix_web::{web, HttpResponse};

use crate::auth::CurrentUser;
use crate::db::pool::pool;
use crate::error::AppResult;
use crate::models::outline::{CreateOutlineNodeRequest, UpdateOutlineNodeRequest};
use crate::services;

pub async fn tree(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let novel_id = path.into_inner();
    let items = services::outline_service::tree(pool(), &user.id, &novel_id).await?;
    Ok(HttpResponse::Ok().json(items))
}

pub async fn create(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<CreateOutlineNodeRequest>,
) -> AppResult<HttpResponse> {
    let novel_id = path.into_inner();
    let n = services::outline_service::create(pool(), &user.id, &novel_id, req.into_inner()).await?;
    Ok(HttpResponse::Created().json(n))
}

pub async fn update(
    user: CurrentUser,
    path: web::Path<String>,
    req: web::Json<UpdateOutlineNodeRequest>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let n = services::outline_service::update(pool(), &user.id, &id, req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(n))
}

pub async fn delete(user: CurrentUser, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    services::outline_service::delete(pool(), &user.id, &id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/novels/{novel_id}/outline")
            .route(web::get().to(tree)),
    );
    cfg.service(
        web::resource("/novels/{novel_id}/outline/nodes")
            .route(web::post().to(create)),
    );
    cfg.service(
        web::resource("/outline/nodes/{id}")
            .route(web::patch().to(update))
            .route(web::delete().to(delete)),
    );
}
