use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::new_id;
use crate::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};
use crate::repositories;

pub async fn list(pool: &SqlitePool, owner_id: &str) -> AppResult<Vec<Project>> {
    repositories::projects::list_by_owner(pool, owner_id).await
}

pub async fn create(
    pool: &SqlitePool,
    owner_id: &str,
    req: CreateProjectRequest,
) -> AppResult<Project> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }
    let now = chrono::Utc::now().to_rfc3339();
    repositories::projects::insert(
        pool,
        &new_id(),
        owner_id,
        name,
        req.description.unwrap_or_default().trim(),
        &now,
    )
    .await
}

pub async fn get(pool: &SqlitePool, owner_id: &str, id: &str) -> AppResult<Project> {
    let p = repositories::projects::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {} not found", id)))?;
    if p.owner_id != owner_id {
        return Err(AppError::Forbidden);
    }
    Ok(p)
}

pub async fn update(
    pool: &SqlitePool,
    owner_id: &str,
    id: &str,
    req: UpdateProjectRequest,
) -> AppResult<Project> {
    get(pool, owner_id, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    repositories::projects::update(
        pool,
        id,
        req.name.as_deref().map(str::trim),
        req.description.as_deref().map(str::trim),
        &now,
    )
    .await
}

pub async fn delete(pool: &SqlitePool, owner_id: &str, id: &str) -> AppResult<()> {
    get(pool, owner_id, id).await?;
    repositories::projects::delete(pool, id).await
}
