use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Character {
    pub id: String,
    pub novel_id: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub traits: String,
    pub backstory: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateCharacterRequest {
    #[schema(example = "林霄")]
    pub name: String,
    #[schema(example = "protagonist")]
    pub role: Option<String>,
    #[schema(example = "18 岁少年，冷静坚毅")]
    pub description: Option<String>,
    #[schema(example = "[\"勇敢\",\"机智\"]")]
    pub traits: Option<String>,
    #[schema(example = "孤儿出身")]
    pub backstory: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateCharacterRequest {
    pub name: Option<String>,
    pub role: Option<String>,
    pub description: Option<String>,
    pub traits: Option<String>,
    pub backstory: Option<String>,
}
