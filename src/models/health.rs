use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 模型服务健康状态
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ModelHealth {
    /// 模型名称
    #[schema(example = "deepseek")]
    pub name: String,
    /// 模型状态
    #[schema(example = "ok")]
    pub status: String,
    /// 错误信息（如果有）
    #[schema(example = "")]
    pub error: String,
}

/// 健康检查响应模型
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct HealthResponse {
    /// 服务状态
    #[schema(example = "ok")]
    pub status: String,
    /// 服务版本
    #[schema(example = "0.1.0")]
    pub version: String,
    /// 模型服务健康状态
    pub models: Vec<ModelHealth>,
    /// 系统时间
    pub timestamp: String,
}
