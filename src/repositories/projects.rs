use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::project::Project;

pub async fn list_by_owner(pool: &SqlitePool, owner_id: &str) -> AppResult<Vec<Project>> {
    let rows = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, owner_id, name, description, created_at, updated_at \
         FROM projects WHERE owner_id = ? ORDER BY created_at DESC",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Project>> {
    let row: Option<ProjectRow> = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, owner_id, name, description, created_at, updated_at \
         FROM projects WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    owner_id: &str,
    name: &str,
    description: &str,
    now: &str,
) -> AppResult<Project> {
    sqlx::query(
        "INSERT INTO projects (id, owner_id, name, description, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(owner_id)
    .bind(name)
    .bind(description)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("project vanished after insert".into()))
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    now: &str,
) -> AppResult<Project> {
    if let Some(n) = name {
        sqlx::query("UPDATE projects SET name = ?, updated_at = ? WHERE id = ?")
            .bind(n)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(d) = description {
        sqlx::query("UPDATE projects SET description = ?, updated_at = ? WHERE id = ?")
            .bind(d)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {} not found", id)))
}

pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let res = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("project {} not found", id)));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: String,
    owner_id: String,
    name: String,
    description: String,
    created_at: String,
    updated_at: String,
}

impl From<ProjectRow> for Project {
    fn from(r: ProjectRow) -> Self {
        Project {
            id: r.id,
            owner_id: r.owner_id,
            name: r.name,
            description: r.description,
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
