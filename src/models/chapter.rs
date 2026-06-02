use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Chapter {
    pub id: String,
    pub novel_id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub order_index: i32,
    pub status: String,
    pub word_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateChapterRequest {
    #[schema(example = "第一章 觉醒")]
    pub title: String,
    #[schema(example = "主角发现自己的龙族血脉")]
    pub summary: Option<String>,
    pub order_index: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateChapterRequest {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub order_index: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ReorderRequest {
    pub target_index: i32,
}
