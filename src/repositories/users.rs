use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::user::User;

pub async fn find_by_email(pool: &SqlitePool, email: &str) -> AppResult<Option<User>> {
    let row: Option<UserRow> = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password_hash, display_name, created_at, updated_at \
         FROM users WHERE email = ?",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<User>> {
    let row: Option<UserRow> = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password_hash, display_name, created_at, updated_at \
         FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    email: &str,
    password_hash: &str,
    display_name: &str,
    now: &str,
) -> AppResult<User> {
    let res = sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await;
    match res {
        Ok(_) => find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::Internal("user vanished after insert".into())),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            Err(AppError::Conflict("email already registered".into()))
        }
        Err(e) => Err(e.into()),
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    email: String,
    password_hash: String,
    display_name: String,
    created_at: String,
    updated_at: String,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id,
            email: r.email,
            password_hash: r.password_hash,
            display_name: r.display_name,
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
