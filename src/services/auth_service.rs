use sqlx::SqlitePool;

use crate::auth::jwt::issue_token;
use crate::auth::password::{hash_password, verify_password};
use crate::error::{AppError, AppResult};
use crate::models::new_id;
use crate::models::user::{AuthResponse, LoginRequest, RegisterRequest, User};
use crate::repositories;

pub async fn register(pool: &SqlitePool, req: RegisterRequest) -> AppResult<AuthResponse> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::Validation("invalid email".into()));
    }
    if req.password.len() < 6 {
        return Err(AppError::Validation("password must be at least 6 characters".into()));
    }
    if req.display_name.trim().is_empty() {
        return Err(AppError::Validation("display_name is required".into()));
    }

    let password_hash = hash_password(&req.password)?;
    let now = chrono::Utc::now().to_rfc3339();
    let user = repositories::users::insert(
        pool,
        &new_id(),
        &email,
        &password_hash,
        req.display_name.trim(),
        &now,
    )
    .await?;

    let token = issue_token(&user.id)?;
    Ok(AuthResponse { token, user })
}

pub async fn login(pool: &SqlitePool, req: LoginRequest) -> AppResult<AuthResponse> {
    let email = req.email.trim().to_lowercase();
    let user = repositories::users::find_by_email(pool, &email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !verify_password(&req.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let token = issue_token(&user.id)?;
    Ok(AuthResponse { token, user })
}

pub async fn me(pool: &SqlitePool, user_id: &str) -> AppResult<User> {
    repositories::users::find_by_id(pool, user_id)
        .await?
        .ok_or(AppError::Unauthorized)
}
