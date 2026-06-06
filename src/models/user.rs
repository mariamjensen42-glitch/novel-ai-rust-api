use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RegisterRequest {
    #[schema(example = "alice@example.com")]
    pub email: String,
    #[schema(example = "super-secret-pw")]
    pub password: String,
    #[schema(example = "Alice")]
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "alice@example.com")]
    pub email: String,
    #[schema(example = "super-secret-pw")]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}
