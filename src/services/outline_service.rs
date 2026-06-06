use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::new_id;
use crate::models::outline::{
    CreateOutlineNodeRequest, OutlineNode, OutlineTreeNode, UpdateOutlineNodeRequest,
};
use crate::repositories;

pub async fn tree(pool: &SqlitePool, owner_id: &str, novel_id: &str) -> AppResult<Vec<OutlineTreeNode>> {
    let novel = repositories::novels::find_by_id(pool, novel_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", novel_id)))?;
    crate::services::novel_service::ensure_owner(pool, owner_id, &novel.project_id).await?;

    let flat = repositories::outlines::list_by_novel(pool, novel_id).await?;
    Ok(build_tree(flat))
}

fn build_tree(flat: Vec<OutlineNode>) -> Vec<OutlineTreeNode> {
    let mut by_parent: HashMap<Option<String>, Vec<OutlineNode>> = HashMap::new();
    for n in flat {
        by_parent.entry(n.parent_id.clone()).or_default().push(n);
    }
    fn construct(
        parent: Option<String>,
        by_parent: &mut HashMap<Option<String>, Vec<OutlineNode>>,
    ) -> Vec<OutlineTreeNode> {
        let mut out = Vec::new();
        if let Some(mut nodes) = by_parent.remove(&parent) {
            nodes.sort_by_key(|n| n.order_index);
            for n in nodes {
                let children = construct(Some(n.id.clone()), by_parent);
                out.push(OutlineTreeNode { node: n, children });
            }
        }
        out
    }
    construct(None, &mut by_parent)
}

pub async fn create(
    pool: &SqlitePool,
    owner_id: &str,
    novel_id: &str,
    req: CreateOutlineNodeRequest,
) -> AppResult<OutlineNode> {
    let novel = repositories::novels::find_by_id(pool, novel_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", novel_id)))?;
    crate::services::novel_service::ensure_owner(pool, owner_id, &novel.project_id).await?;
    if req.title.trim().is_empty() {
        return Err(AppError::Validation("title is required".into()));
    }
    if let Some(pid) = &req.parent_id {
        let parent = repositories::outlines::find_by_id(pool, pid).await?.ok_or_else(|| {
            AppError::NotFound(format!("parent outline node {} not found", pid))
        })?;
        if parent.novel_id != novel_id {
            return Err(AppError::Validation("parent belongs to a different novel".into()));
        }
    }
    let order_index = req.order_index.unwrap_or(0);
    let now = chrono::Utc::now().to_rfc3339();
    repositories::outlines::insert(
        pool,
        &new_id(),
        novel_id,
        req.parent_id.as_deref(),
        req.title.trim(),
        req.summary.unwrap_or_default().trim(),
        order_index,
        req.chapter_id.as_deref(),
        &now,
    )
    .await
}

pub async fn update(
    pool: &SqlitePool,
    owner_id: &str,
    id: &str,
    req: UpdateOutlineNodeRequest,
) -> AppResult<OutlineNode> {
    let n = repositories::outlines::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("outline node {} not found", id)))?;
    let novel = repositories::novels::find_by_id(pool, &n.novel_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", n.novel_id)))?;
    crate::services::novel_service::ensure_owner(pool, owner_id, &novel.project_id).await?;
    let chapter_id_param = req.chapter_id.as_deref();
    let now = chrono::Utc::now().to_rfc3339();
    repositories::outlines::update(
        pool,
        id,
        req.title.as_deref().map(str::trim),
        req.summary.as_deref().map(str::trim),
        req.order_index,
        chapter_id_param,
        &now,
    )
    .await
}

pub async fn delete(pool: &SqlitePool, owner_id: &str, id: &str) -> AppResult<()> {
    let n = repositories::outlines::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("outline node {} not found", id)))?;
    let novel = repositories::novels::find_by_id(pool, &n.novel_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", n.novel_id)))?;
    crate::services::novel_service::ensure_owner(pool, owner_id, &novel.project_id).await?;
    repositories::outlines::delete(pool, id).await
}
