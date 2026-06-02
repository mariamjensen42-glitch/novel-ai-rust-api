use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::novel::Novel;

pub async fn list_by_project(pool: &SqlitePool, project_id: &str) -> AppResult<Vec<Novel>> {
    let rows = sqlx::query_as::<_, NovelRow>(
        "SELECT id, project_id, title, synopsis, genre, style, pov, tone, target_word_count, \
         created_at, updated_at \
         FROM novels WHERE project_id = ? ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Novel>> {
    let row: Option<NovelRow> = sqlx::query_as::<_, NovelRow>(
        "SELECT id, project_id, title, synopsis, genre, style, pov, tone, target_word_count, \
         created_at, updated_at \
         FROM novels WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    project_id: &str,
    title: &str,
    synopsis: &str,
    genre: &str,
    style: &str,
    pov: &str,
    tone: &str,
    target_word_count: i32,
    now: &str,
) -> AppResult<Novel> {
    sqlx::query(
        "INSERT INTO novels (id, project_id, title, synopsis, genre, style, pov, tone, \
         target_word_count, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(project_id)
    .bind(title)
    .bind(synopsis)
    .bind(genre)
    .bind(style)
    .bind(pov)
    .bind(tone)
    .bind(target_word_count)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("novel vanished after insert".into()))
}

#[allow(clippy::too_many_arguments)]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    title: Option<&str>,
    synopsis: Option<&str>,
    genre: Option<&str>,
    style: Option<&str>,
    pov: Option<&str>,
    tone: Option<&str>,
    target_word_count: Option<i32>,
    now: &str,
) -> AppResult<Novel> {
    if let Some(v) = title {
        sqlx::query("UPDATE novels SET title = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = synopsis {
        sqlx::query("UPDATE novels SET synopsis = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = genre {
        sqlx::query("UPDATE novels SET genre = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = style {
        sqlx::query("UPDATE novels SET style = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = pov {
        sqlx::query("UPDATE novels SET pov = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = tone {
        sqlx::query("UPDATE novels SET tone = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = target_word_count {
        sqlx::query("UPDATE novels SET target_word_count = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("novel {} not found", id)))
}

pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let res = sqlx::query("DELETE FROM novels WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("novel {} not found", id)));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct NovelRow {
    id: String,
    project_id: String,
    title: String,
    synopsis: String,
    genre: String,
    style: String,
    pov: String,
    tone: String,
    target_word_count: i32,
    created_at: String,
    updated_at: String,
}

impl From<NovelRow> for Novel {
    fn from(r: NovelRow) -> Self {
        Novel {
            id: r.id,
            project_id: r.project_id,
            title: r.title,
            synopsis: r.synopsis,
            genre: r.genre,
            style: r.style,
            pov: r.pov,
            tone: r.tone,
            target_word_count: r.target_word_count,
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
