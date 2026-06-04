//! 自定义中间件
//!
//! 当前包含：
//! - [`request_id`] —— RequestId 注入、回传、日志串联
//!
//! 用法：`actix_web::middleware::from_fn(request_id::middleware)`

pub mod request_id;
