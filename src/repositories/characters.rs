use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::character::Character;

pub async fn list_by_novel(pool: &SqlitePool, novel_id: &str) -> AppResult<Vec<Character>> {
    let rows = sqlx::query_as::<_, CharacterRow>(
        "SELECT id, novel_id, name, role, description, traits, backstory, created_at, updated_at \
         FROM characters WHERE novel_id = ? ORDER BY created_at ASC",
    )
    .bind(novel_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Character>> {
    let row: Option<CharacterRow> = sqlx::query_as::<_, CharacterRow>(
        "SELECT id, novel_id, name, role, description, traits, backstory, created_at, updated_at \
         FROM characters WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn find_many_by_ids(
    pool: &SqlitePool,
    ids: &[String],
) -> AppResult<Vec<Character>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let placeholders = std::iter::repeat("?")
        .take(ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT id, novel_id, name, role, description, traits, backstory, created_at, updated_at \
         FROM characters WHERE id IN ({})",
        placeholders
    );
    let mut q = sqlx::query_as::<_, CharacterRow>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(pool).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    novel_id: &str,
    name: &str,
    role: &str,
    description: &str,
    traits: &str,
    backstory: &str,
    now: &str,
) -> AppResult<Character> {
    sqlx::query(
        "INSERT INTO characters (id, novel_id, name, role, description, traits, backstory, \
         created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(novel_id)
    .bind(name)
    .bind(role)
    .bind(description)
    .bind(traits)
    .bind(backstory)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("character vanished after insert".into()))
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    role: Option<&str>,
    description: Option<&str>,
    traits: Option<&str>,
    backstory: Option<&str>,
    now: &str,
) -> AppResult<Character> {
    if let Some(v) = name {
        sqlx::query("UPDATE characters SET name = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = role {
        sqlx::query("UPDATE characters SET role = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = description {
        sqlx::query("UPDATE characters SET description = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = traits {
        sqlx::query("UPDATE characters SET traits = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(v) = backstory {
        sqlx::query("UPDATE characters SET backstory = ?, updated_at = ? WHERE id = ?")
            .bind(v)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("character {} not found", id)))
}

pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let res = sqlx::query("DELETE FROM characters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("character {} not found", id)));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct CharacterRow {
    id: String,
    novel_id: String,
    name: String,
    role: String,
    description: String,
    traits: String,
    backstory: String,
    created_at: String,
    updated_at: String,
}

impl From<CharacterRow> for Character {
    fn from(r: CharacterRow) -> Self {
        Character {
            id: r.id,
            novel_id: r.novel_id,
            name: r.name,
            role: r.role,
            description: r.description,
            traits: r.traits,
            backstory: r.backstory,
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
