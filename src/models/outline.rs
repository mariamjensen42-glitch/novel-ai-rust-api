use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OutlineNode {
    pub id: String,
    pub novel_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub order_index: i32,
    pub chapter_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OutlineTreeNode {
    #[serde(flatten)]
    pub node: OutlineNode,
    pub children: Vec<OutlineTreeNode>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateOutlineNodeRequest {
    pub title: String,
    pub summary: Option<String>,
    pub parent_id: Option<String>,
    pub order_index: Option<i32>,
    pub chapter_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateOutlineNodeRequest {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub order_index: Option<i32>,
    pub chapter_id: Option<String>,
}
