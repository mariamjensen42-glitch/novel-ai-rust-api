use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::chapter::{Chapter, CreateChapterRequest, ReorderRequest, UpdateChapterRequest};
use crate::models::new_id;
use crate::repositories;

pub async fn list_by_novel(
    pool: &SqlitePool,
    owner_id: &str,
    novel_id: &str,
) -> AppResult<Vec<Chapter>> {
    let novel = repositories::novels::find_by_id(pool, novel_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", novel_id)))?;
    crate::services::novel_service::ensure_owner(pool, owner_id, &novel.project_id).await?;
    repositories::chapters::list_by_novel(pool, novel_id).await
}

pub async fn create(
    pool: &SqlitePool,
    owner_id: &str,
    novel_id: &str,
    req: CreateChapterRequest,
) -> AppResult<Chapter> {
    let novel = repositories::novels::find_by_id(pool, novel_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", novel_id)))?;
    crate::services::novel_service::ensure_owner(pool, owner_id, &novel.project_id).await?;
    let title = req.title.trim();
    if title.is_empty() {
        return Err(AppError::Validation("title is required".into()));
    }
    let order_index = match req.order_index {
        Some(v) => v,
        None => repositories::chapters::next_order_index(pool, novel_id).await?,
    };
    let now = chrono::Utc::now().to_rfc3339();
    repositories::chapters::insert(
        pool,
        &new_id(),
        novel_id,
        title,
        req.summary.unwrap_or_default().trim(),
        order_index,
        &now,
    )
    .await
}

pub async fn get(pool: &SqlitePool, owner_id: &str, id: &str) -> AppResult<Chapter> {
    let c = repositories::chapters::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("chapter {} not found", id)))?;
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
    req: UpdateChapterRequest,
) -> AppResult<Chapter> {
    let c = get(pool, owner_id, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    repositories::chapters::update(
        pool,
        &c.id,
        req.title.as_deref().map(str::trim),
        req.summary.as_deref().map(str::trim),
        req.content.as_deref(),
        req.order_index,
        req.status.as_deref(),
        &now,
    )
    .await
}

pub async fn delete(pool: &SqlitePool, owner_id: &str, id: &str) -> AppResult<()> {
    get(pool, owner_id, id).await?;
    repositories::chapters::delete(pool, id).await
}

pub async fn reorder(
    pool: &SqlitePool,
    owner_id: &str,
    id: &str,
    req: ReorderRequest,
) -> AppResult<Chapter> {
    let c = get(pool, owner_id, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    repositories::chapters::update(
        pool,
        &c.id,
        None,
        None,
        None,
        Some(req.target_index),
        None,
        &now,
    )
    .await
}

/// AI 写完后回写：先做权限校验，再覆写 content / summary。
/// 这是一个薄包装；如果之后接入版本快照系统，只需在此函数内增加快照写入，
/// 所有调用方（generation_service 的 run_*）无需修改。
#[allow(clippy::too_many_arguments)]
pub async fn write_ai_result(
    pool: &SqlitePool,
    owner_id: &str,
    id: &str,
    new_content: &str,
    new_summary: Option<&str>,
    _action: &str,
    _note: &str,
) -> AppResult<Chapter> {
    let c = get(pool, owner_id, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    repositories::chapters::update(
        pool,
        &c.id,
        None,
        new_summary,
        Some(new_content),
        None,
        None,
        &now,
    )
    .await
}
