//! RequestId 中间件
//!
//! 行为：
//! - 优先继承上游 `X-Request-Id` 头（合法时）
//! - 否则生成 UUID v4
//! - 写入 `extensions` 供 handler 提取
//! - 通过子 span `info_span!("request", request_id = %id)` 让 handler 内所有日志自动继承
//! - 响应头回传，方便客户端排障
//!
//! ID 白名单：`[a-zA-Z0-9-]{8,64}`，防止日志注入（`\n` 拆日志行）和 Header 注入（`\r\n` 拆头）。

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::middleware::Next;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use tracing::Instrument;
use uuid::Uuid;

/// HTTP 头名称（小写）
pub const HEADER: &str = "x-request-id";

/// 请求 ID 包装类型，从 `extensions` 提取
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl FromRequest for RequestId {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &HttpRequest,
        _: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let id = req
            .extensions()
            .get::<RequestId>()
            .cloned()
            .unwrap_or_else(|| RequestId("unknown".into()));
        std::future::ready(Ok(id))
    }
}

/// 中间件函数本体
pub async fn middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let id = req
        .headers()
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| is_valid_id(s))
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    req.extensions_mut().insert(RequestId(id.clone()));

    // 创建子 span，handler 内所有日志自动携带 request_id 字段
    let span = tracing::info_span!("request", request_id = %id);
    let mut res = next.call(req).instrument(span).await?;

    if let Ok(hv) = HeaderValue::from_str(&id) {
        res.headers_mut()
            .insert(HeaderName::from_static(HEADER), hv);
    }
    Ok(res)
}

/// 校验 ID 是否合法白名单格式
pub fn is_valid_id(s: &str) -> bool {
    let len = s.len();
    (8..=64).contains(&len) && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_uuid() {
        let id = Uuid::new_v4().to_string();
        assert!(is_valid_id(&id));
    }

    #[test]
    fn valid_simple() {
        assert!(is_valid_id("my-trace-001"));
        assert!(is_valid_id("abcdefgh"));
    }

    #[test]
    fn too_short_rejected() {
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("abc"));
        assert!(!is_valid_id("1234567"));
    }

    #[test]
    fn too_long_rejected() {
        let s: String = "a".repeat(65);
        assert!(!is_valid_id(&s));
    }

    #[test]
    fn injection_chars_rejected() {
        // Header injection
        assert!(!is_valid_id("abc\r\ndef-injection"));
        // Log injection
        assert!(!is_valid_id("abc\ndef"));
        assert!(!is_valid_id("abc def"));
        // Special chars
        assert!(!is_valid_id("abc/def"));
        assert!(!is_valid_id("abc.def"));
        assert!(!is_valid_id("中文ID")); // 非 ASCII
    }

    #[test]
    fn hyphens_and_underscores_alphanumeric_only() {
        // underscore 不在白名单
        assert!(!is_valid_id("trace_001"));
        // hyphen 允许
        assert!(is_valid_id("trace-001-aaa"));
    }
}
