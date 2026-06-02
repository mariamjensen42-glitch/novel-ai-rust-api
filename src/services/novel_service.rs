use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::new_id;
use crate::models::novel::{CreateNovelRequest, Novel, UpdateNovelRequest};
use crate::repositories;

pub async fn list_by_project(
    pool: &SqlitePool,
    owner_id: &str,
    project_id: &str,
) -> AppResult<Vec<Novel>> {
    let project = repositories::projects::find_by_id(pool, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {} not found", project_id)))?;
    if project.owner_id != owner_id {
        return Err(AppError::Forbidden);
    }
    repositories::novels::list_by_project(pool, project_id).await
}

pub async fn create(
    pool: &SqlitePool,
    owner_id: &str,
    project_id: &str,
    req: CreateNovelRequest,
) -> AppResult<Novel> {
    let project = repositories::projects::find_by_id(pool, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {} not found", project_id)))?;
    if project.owner_id != owner_id {
        return Err(AppError::Forbidden);
    }
    let title = req.title.trim();
    if title.is_empty() {
        return Err(AppError::Validation("title is required".into()));
    }
    let now = chrono::Utc::now().to_rfc3339();
    repositories::novels::insert(
        pool,
        &new_id(),
        project_id,
        title,
        req.synopsis.unwrap_or_default().trim(),
        req.genre.unwrap_or_default().trim(),
        req.style.unwrap_or_default().trim(),
        req.pov.unwrap_or_else(|| "third".into()).trim(),
        req.tone.unwrap_or_default().trim(),
        req.target_word_count.unwrap_or(0),
        &now,
    )
    .await
}

pub async fn get(pool: &SqlitePool, owner_id: &str, id: &str) -> AppResult<Novel> {
    let n = repositories::novels::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", id)))?;
    ensure_owner(pool, owner_id, &n.project_id).await?;
    Ok(n)
}

pub async fn update(
    pool: &SqlitePool,
    owner_id: &str,
    id: &str,
    req: UpdateNovelRequest,
) -> AppResult<Novel> {
    let n = get(pool, owner_id, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    repositories::novels::update(
        pool,
        &n.id,
        req.title.as_deref().map(str::trim),
        req.synopsis.as_deref().map(str::trim),
        req.genre.as_deref().map(str::trim),
        req.style.as_deref().map(str::trim),
        req.pov.as_deref().map(str::trim),
        req.tone.as_deref().map(str::trim),
        req.target_word_count,
        &now,
    )
    .await
}

pub async fn delete(pool: &SqlitePool, owner_id: &str, id: &str) -> AppResult<()> {
    get(pool, owner_id, id).await?;
    repositories::novels::delete(pool, id).await
}

pub async fn ensure_owner(pool: &SqlitePool, owner_id: &str, project_id: &str) -> AppResult<()> {
    let p = repositories::projects::find_by_id(pool, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {} not found", project_id)))?;
    if p.owner_id != owner_id {
        return Err(AppError::Forbidden);
    }
    Ok(())
}
