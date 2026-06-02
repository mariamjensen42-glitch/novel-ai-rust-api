use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::outline::OutlineNode;

pub async fn list_by_novel(pool: &SqlitePool, novel_id: &str) -> AppResult<Vec<OutlineNode>> {
    let rows = sqlx::query_as::<_, OutlineRow>(
        "SELECT id, novel_id, parent_id, title, summary, order_index, chapter_id, \
         created_at, updated_at \
         FROM outline_nodes WHERE novel_id = ? ORDER BY order_index ASC",
    )
    .bind(novel_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<OutlineNode>> {
    let row: Option<OutlineRow> = sqlx::query_as::<_, OutlineRow>(
        "SELECT id, novel_id, parent_id, title, summary, order_index, chapter_id, \
         created_at, updated_at FROM outline_nodes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    novel_id: &str,
    parent_id: Option<&str>,
    title: &str,
    summary: &str,
    order_index: i32,
    chapter_id: Option<&str>,
    now: &str,
) -> AppResult<OutlineNode> {
    sqlx::query(
        "INSERT INTO outline_nodes (id, novel_id, parent_id, title, summary, order_index, \
         chapter_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(novel_id)
    .bind(parent_id)
    .bind(title)
    .bind(summary)
    .bind(order_index)
    .bind(chapter_id)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("outline node vanished after insert".into()))
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    title: Option<&str>,
    summary: Option<&str>,
    order_index: Option<i32>,
    chapter_id: Option<&str>,
    now: &str,
) -> AppResult<OutlineNode> {
    if let Some(v) = title {
        sqlx::query("UPDATE outline_nodes SET title = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = summary {
        sqlx::query("UPDATE outline_nodes SET summary = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = order_index {
        sqlx::query("UPDATE outline_nodes SET order_index = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = chapter_id {
        sqlx::query("UPDATE outline_nodes SET chapter_id = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("outline node {} not found", id)))
}

pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let res = sqlx::query("DELETE FROM outline_nodes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("outline node {} not found", id)));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct OutlineRow {
    id: String,
    novel_id: String,
    parent_id: Option<String>,
    title: String,
    summary: String,
    order_index: i32,
    chapter_id: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<OutlineRow> for OutlineNode {
    fn from(r: OutlineRow) -> Self {
        OutlineNode {
            id: r.id,
            novel_id: r.novel_id,
            parent_id: r.parent_id,
            title: r.title,
            summary: r.summary,
            order_index: r.order_index,
            chapter_id: r.chapter_id,
            created_at: parse_dt(&r.created_at),
            updated_at: parse_dt(&r.updated_at),
        }
    }
}

fn parse_dt(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}
