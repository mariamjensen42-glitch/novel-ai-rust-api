//! 后台任务 panic 兜底
//!
//! 替换裸 `tokio::spawn`，避免单次 panic 让整个 SSE 流半挂。
//! 提供两个变体：
//! - [`spawn_catch`]：通用版本，仅记日志 + 增加 `tasks_panicked_total` 计数
//! - [`spawn_catch_with_tx`]：SSE 专用，panic 时给客户端发一个脱敏的 `Error` 事件

use std::future::Future;
use std::panic::AssertUnwindSafe;

use futures_util::FutureExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::error::AppResult;
use crate::observability::metrics::TASKS_PANICKED_TOTAL;
use crate::observability::redact::public_message;
use crate::sse::SsePayload;

/// 把 panic payload 提取成 `String`（不直接 `format!("{:?}", ...)`，避免泄漏大块内存）
pub fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&'static str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "<non-string panic payload>".into())
}

/// 通用版本：panic 时记日志 + 增计数，不外抛
pub fn spawn_catch<F>(name: &'static str, fut: F) -> JoinHandle<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        let outcome = AssertUnwindSafe(fut).catch_unwind().await;
        if let Err(payload) = outcome {
            let msg = panic_message(&payload);
            tracing::error!(task = name, panic = %msg, "background task panicked");
            TASKS_PANICKED_TOTAL.with_label_values(&[name]).inc();
        }
    })
}

/// SSE 专用：panic 时给客户端发脱敏 Error 事件；正常返回 `Err` 时也发
pub fn spawn_catch_with_tx<F>(
    name: &'static str,
    tx: mpsc::Sender<SsePayload>,
    fut: F,
) -> JoinHandle<()>
where
    F: Future<Output = AppResult<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let outcome = AssertUnwindSafe(fut).catch_unwind().await;
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                tracing::warn!(task = name, error = ?e, "background task returned error");
                let _ = tx.send(SsePayload::Error { message: public_message(&e) }).await;
            }
            Err(payload) => {
                let msg = panic_message(&payload);
                tracing::error!(task = name, panic = %msg, "background task panicked");
                TASKS_PANICKED_TOTAL.with_label_values(&[name]).inc();
                let _ = tx
                    .send(SsePayload::Error { message: "internal error".into() })
                    .await;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::AssertUnwindSafe;

    // 注意：这里的测试**不**走 spawn，直接用 catch_unwind 验证 panic_message
    // 行为。spawn_catch 的 async 行为通过集成测试覆盖。

    #[test]
    fn panic_message_from_str() {
        let result = std::panic::catch_unwind(|| {
            panic!("boom: {}", 42);
        });
        let payload = result.unwrap_err();
        let msg = panic_message(&payload);
        assert!(msg.contains("boom"));
    }

    #[test]
    fn panic_message_from_string() {
        let result = std::panic::catch_unwind(|| {
            let s: String = "explicit string panic".to_string();
            std::panic::panic_any(s);
        });
        let payload = result.unwrap_err();
        let msg = panic_message(&payload);
        assert_eq!(msg, "explicit string panic");
    }

    #[test]
    fn panic_message_unknown_payload() {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            std::panic::panic_any(42i32);
        }));
        let payload = result.unwrap_err();
        let msg = panic_message(&payload);
        assert_eq!(msg, "<non-string panic payload>");
    }
}
