use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::character::{Character, CreateCharacterRequest, UpdateCharacterRequest};
use crate::models::new_id;
use crate::repositories;

pub async fn list_by_novel(
    pool: &SqlitePool,
    owner_id: &str,
    novel_id: &str,
) -> AppResult<Vec<Character>> {
    let novel = repositories::novels::find_by_id(pool, novel_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", novel_id)))?;
    crate::services::novel_service::ensure_owner(pool, owner_id, &novel.project_id).await?;
    repositories::characters::list_by_novel(pool, novel_id).await
}

pub async fn create(
    pool: &SqlitePool,
    owner_id: &str,
    novel_id: &str,
    req: CreateCharacterRequest,
) -> AppResult<Character> {
    let novel = repositories::novels::find_by_id(pool, novel_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", novel_id)))?;
    crate::services::novel_service::ensure_owner(pool, owner_id, &novel.project_id).await?;
    if req.name.trim().is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }
    let now = chrono::Utc::now().to_rfc3339();
    repositories::characters::insert(
        pool,
        &new_id(),
        novel_id,
        req.name.trim(),
        req.role.unwrap_or_else(|| "supporting".into()).trim(),
        req.description.unwrap_or_default().trim(),
        req.traits.unwrap_or_default().trim(),
        req.backstory.unwrap_or_default().trim(),
        &now,
    )
    .await
}

pub async fn get(pool: &SqlitePool, owner_id: &str, id: &str) -> AppResult<Character> {
    let c = repositories::characters::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("character {} not found", id)))?;
    let novel = repositories::novels::find_by_id(pool, &c.novel_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", c.novel_id)))?;
    crate::services::novel_service::ensure_owner(pool, owner_id, &novel.project_id).await?;
    Ok(c)
}

pub async fn update(
    pool: &SqlitePool,
    owner_id: &str,
    id: &str,
    req: UpdateCharacterRequest,
) -> AppResult<Character> {
    let c = get(pool, owner_id, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    repositories::characters::update(
        pool,
        &c.id,
        req.name.as_deref().map(str::trim),
        req.role.as_deref().map(str::trim),
        req.description.as_deref().map(str::trim),
        req.traits.as_deref().map(str::trim),
        req.backstory.as_deref().map(str::trim),
        &now,
    )
    .await
}

pub async fn delete(pool: &SqlitePool, owner_id: &str, id: &str) -> AppResult<()> {
    get(pool, owner_id, id).await?;
    repositories::characters::delete(pool, id).await
}
