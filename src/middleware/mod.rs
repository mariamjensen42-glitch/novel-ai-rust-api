//! 自定义中间件
//!
//! 当前包含：
//! - [`request_id`] —— RequestId 注入、回传、日志串联

pub mod request_id;

pub use request_id::{middleware as request_id_middleware, RequestId, HEADER as REQUEST_ID_HEADER};
