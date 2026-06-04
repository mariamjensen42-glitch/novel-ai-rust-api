//! 错误响应脱敏：把内部细节（SQL 错误、内部 IP、上游 API 形态）隐藏在
//! 后端日志中，只返给前端一个简短、安全的提示。
//!
//! 业务级错误（Validation/Conflict/NotFound）保留原文，便于前端调 bug。
//! 内部错误（Database/Internal/LlmUpstream）统一成泛化文案。

use crate::error::AppError;

/// 返回给前端的"安全"错误消息
///
/// 业务错误：原文透传（用户能看懂、能调）
/// 内部错误：泛化文案（防泄漏 schema、内部地址、第三方 API 形态）
pub fn public_message(err: &AppError) -> String {
    match err {
        AppError::Validation(m) => m.clone(),
        AppError::Unauthorized => "authentication required".into(),
        AppError::Forbidden => "permission denied".into(),
        AppError::NotFound(_) => "resource not found".into(),
        AppError::Conflict(m) => m.clone(),
        AppError::LlmUpstream(_) => "upstream service error".into(),
        AppError::Database(_) | AppError::Internal(_) => "internal error".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    #[test]
    fn validation_keeps_message() {
        let e = AppError::Validation("email invalid".into());
        assert_eq!(public_message(&e), "email invalid");
    }

    #[test]
    fn unauthorized_is_generic() {
        assert_eq!(public_message(&AppError::Unauthorized), "authentication required");
    }

    #[test]
    fn forbidden_is_generic() {
        assert_eq!(public_message(&AppError::Forbidden), "permission denied");
    }

    #[test]
    fn not_found_is_generic() {
        let e = AppError::NotFound("chapter_xyz".into());
        let msg = public_message(&e);
        assert_eq!(msg, "resource not found");
        // 关键：原始 ID 不能泄漏给前端
        assert!(!msg.contains("chapter_xyz"));
    }

    #[test]
    fn conflict_keeps_message() {
        let e = AppError::Conflict("email already exists".into());
        assert_eq!(public_message(&e), "email already exists");
    }

    #[test]
    fn llm_upstream_is_redacted() {
        // 模拟原始错误含上游 API 形态
        let raw = "upstream llm error: 401 Unauthorized: {\"error\":{\"message\":\"Invalid API key sk-xxx\"}}";
        let e = AppError::LlmUpstream(raw.into());
        let msg = public_message(&e);
        assert_eq!(msg, "upstream service error");
        assert!(!msg.contains("401"));
        assert!(!msg.contains("sk-xxx"));
        assert!(!msg.contains("Authorization"));
    }

    #[test]
    fn database_is_redacted() {
        // 模拟原始错误含 SQL 细节
        let raw = "database error: UNIQUE constraint failed: users.email";
        // sqlx::Error::from_sql 不能直接构造，但 AppError::Internal 可模拟同样形态
        let e = AppError::Internal(raw.into());
        let msg = public_message(&e);
        assert_eq!(msg, "internal error");
        assert!(!msg.contains("UNIQUE"));
        assert!(!msg.contains("users.email"));
    }
}
