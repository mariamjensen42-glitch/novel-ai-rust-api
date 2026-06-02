use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::config::get_config;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

pub fn issue_token(user_id: &str) -> Result<String, AppError> {
    let cfg = get_config();
    let now = Utc::now();
    let exp = now + Duration::hours(cfg.jwt_ttl_hours);
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(cfg.jwt_secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_token(token: &str) -> Result<Claims, AppError> {
    let cfg = get_config();
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}
