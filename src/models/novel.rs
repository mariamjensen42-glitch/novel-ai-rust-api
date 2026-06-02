use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Novel {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub synopsis: String,
    pub genre: String,
    pub style: String,
    pub pov: String,
    pub tone: String,
    pub target_word_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateNovelRequest {
    #[schema(example = "龙之血脉")]
    pub title: String,
    #[schema(example = "少年在龙族血脉觉醒后踏上征途")]
    pub synopsis: Option<String>,
    #[schema(example = "玄幻")]
    pub genre: Option<String>,
    #[schema(example = "网文")]
    pub style: Option<String>,
    #[schema(example = "third")]
    pub pov: Option<String>,
    #[schema(example = "热血")]
    pub tone: Option<String>,
    pub target_word_count: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateNovelRequest {
    pub title: Option<String>,
    pub synopsis: Option<String>,
    pub genre: Option<String>,
    pub style: Option<String>,
    pub pov: Option<String>,
    pub tone: Option<String>,
    pub target_word_count: Option<i32>,
}
