//! 可观测性核心模块
//!
//! 提供：
//! - `metrics` —— Prometheus 指标注册表 + 11 个业务指标
//! - `redact` —— 错误响应脱敏工具（防泄漏 SQL 细节、内部 IP、上游 API 形态）
//! - `panic_guard` —— 后台任务 panic 兜底（`spawn_catch` / `spawn_catch_with_tx`）
//!
//! 用法见 [docs/superpowers/specs/2026-06-03-observability-design.md](https://example.com/spec)。

pub mod metrics;
pub mod panic_guard;
pub mod redact;

pub use redact::public_message;
