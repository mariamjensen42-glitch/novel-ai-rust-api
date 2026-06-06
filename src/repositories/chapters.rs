use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::chapter::Chapter;

pub async fn list_by_novel(pool: &SqlitePool, novel_id: &str) -> AppResult<Vec<Chapter>> {
    let rows = sqlx::query_as::<_, ChapterRow>(
        "SELECT id, novel_id, title, summary, content, order_index, status, word_count, \
         created_at, updated_at \
         FROM chapters WHERE novel_id = ? ORDER BY order_index ASC",
    )
    .bind(novel_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Chapter>> {
    let row: Option<ChapterRow> = sqlx::query_as::<_, ChapterRow>(
        "SELECT id, novel_id, title, summary, content, order_index, status, word_count, \
         created_at, updated_at \
         FROM chapters WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn next_order_index(pool: &SqlitePool, novel_id: &str) -> AppResult<i32> {
    let row: Option<(Option<i32>,)> = sqlx::query_as(
        "SELECT MAX(order_index) FROM chapters WHERE novel_id = ?",
    )
    .bind(novel_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|(v,)| v).unwrap_or(-1) + 1)
}

pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    novel_id: &str,
    title: &str,
    summary: &str,
    order_index: i32,
    now: &str,
) -> AppResult<Chapter> {
    sqlx::query(
        "INSERT INTO chapters (id, novel_id, title, summary, content, order_index, status, \
         word_count, created_at, updated_at) \
         VALUES (?, ?, ?, ?, '', ?, 'draft', 0, ?, ?)",
    )
    .bind(id)
    .bind(novel_id)
    .bind(title)
    .bind(summary)
    .bind(order_index)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::Conflict("order_index already used in this novel".into())
        }
        other => other.into(),
    })?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("chapter vanished after insert".into()))
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    title: Option<&str>,
    summary: Option<&str>,
    content: Option<&str>,
    order_index: Option<i32>,
    status: Option<&str>,
    now: &str,
) -> AppResult<Chapter> {
    if let Some(v) = title {
        sqlx::query("UPDATE chapters SET title = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = summary {
        sqlx::query("UPDATE chapters SET summary = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = content {
        let wc = count_words(v) as i32;
        sqlx::query("UPDATE chapters SET content = ?, word_count = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(wc)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = order_index {
        sqlx::query("UPDATE chapters SET order_index = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = status {
        sqlx::query("UPDATE chapters SET status = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("chapter {} not found", id)))
}

pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let res = sqlx::query("DELETE FROM chapters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("chapter {} not found", id)));
    }
    Ok(())
}

pub async fn append_content(
    pool: &SqlitePool,
    id: &str,
    extra: &str,
    now: &str,
) -> AppResult<Chapter> {
    let ch = find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("chapter {} not found", id)))?;
    let new_content = if ch.content.is_empty() {
        extra.to_string()
    } else {
        format!("{}{}", ch.content, extra)
    };
    update(pool, id, None, None, Some(&new_content), None, None, now).await
}

fn count_words(s: &str) -> usize {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .filter(|c| c.is_whitespace() == false)
        .count()
}

#[derive(sqlx::FromRow)]
struct ChapterRow {
    id: String,
    novel_id: String,
    title: String,
    summary: String,
    content: String,
    order_index: i32,
    status: String,
    word_count: i32,
    created_at: String,
    updated_at: String,
}

impl From<ChapterRow> for Chapter {
    fn from(r: ChapterRow) -> Self {
        Chapter {
            id: r.id,
            novel_id: r.novel_id,
            title: r.title,
            summary: r.summary,
            content: r.content,
            order_index: r.order_index,
            status: r.status,
            word_count: r.word_count,
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
