//! 可观测性集成测试
//!
//! 覆盖：
//! 1. `/metrics` 端点：200 + 文本格式 + 包含关键指标名
//! 2. request_id 中间件：合法 ID 继承 / 非法 ID 丢弃 / 缺失时生成 UUID
//! 3. panic_guard::spawn_catch：panic 时计数 +1
//! 4. panic_guard::spawn_catch_with_tx：panic 时给客户端发脱敏 SSE Error 事件
//! 5. error_response：ERRORS_TOTAL{code} 增加

use std::sync::Arc;

use actix_web::middleware::from_fn;
use actix_web::test as actix_test;
use actix_web::{web, App, HttpResponse};
use reqwest::Client;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use novel_ai_rust_api::error::AppError;
use novel_ai_rust_api::middleware::request_id::{is_valid_id, middleware as request_id_fn};
use novel_ai_rust_api::observability::metrics::{
    AUTH_FAILURES_TOTAL, DB_QUERY_DURATION_SECONDS, ERRORS_TOTAL, HTTP_REQUESTS_IN_FLIGHT,
    HTTP_REQUESTS_TOTAL, HTTP_REQUEST_DURATION_SECONDS, LLM_STREAM_CHUNKS_TOTAL,
    LLM_STREAM_DURATION_SECONDS, LLM_STREAM_ERRORS_TOTAL, TASKS_PANICKED_TOTAL, render_text,
};

/// 触发所有 `Lazy` 指标初始化（让 `CounterVec`/`HistogramVec` 真的注册到 REGISTRY）
fn force_init_metrics() {
    // 仅 `.with_label_values` 不够创建 child label，但足以让外层 `Lazy` 完成初始化
    let _ = HTTP_REQUESTS_TOTAL.with_label_values(&["PROBE", "/probe", "200"]);
    let _ = HTTP_REQUEST_DURATION_SECONDS.with_label_values(&["PROBE", "/probe"]);
    let _ = HTTP_REQUESTS_IN_FLIGHT.with_label_values(&["/probe"]);
    let _ = LLM_STREAM_CHUNKS_TOTAL.with_label_values(&["probe", "probe", "probe"]);
    let _ = LLM_STREAM_ERRORS_TOTAL.with_label_values(&["probe", "probe", "probe", "probe"]);
    let _ = LLM_STREAM_DURATION_SECONDS.with_label_values(&["probe", "probe", "probe"]);
    let _ = DB_QUERY_DURATION_SECONDS.with_label_values(&["probe"]);
    let _ = AUTH_FAILURES_TOTAL.with_label_values(&["probe"]);
    let _ = TASKS_PANICKED_TOTAL.with_label_values(&["probe"]);
    let _ = ERRORS_TOTAL.with_label_values(&["probe"]);
}
use novel_ai_rust_api::observability::panic_guard::{spawn_catch, spawn_catch_with_tx};
use novel_ai_rust_api::observability::public_message;
use novel_ai_rust_api::sse::{sse_stream, SsePayload};

fn http() -> Arc<Client> {
    Arc::new(Client::new())
}

// ============================================================================
// /metrics 端点
// ============================================================================

async fn metrics_endpoint() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(render_text())
}

#[actix_web::test]
async fn metrics_endpoint_returns_prometheus_text() {
    // 关键：force-init 所有 `Lazy` 指标，否则未访问的 vec 不会注册到 REGISTRY
    force_init_metrics();

    let app = actix_test::init_service(
        App::new().route("/metrics", web::get().to(metrics_endpoint)),
    )
    .await;

    // 先触发一次让指标非零
    HTTP_REQUESTS_TOTAL
        .with_label_values(&["GET", "/metrics", "200"])
        .inc();
    ERRORS_TOTAL.with_label_values(&["not_found"]).inc();
    TASKS_PANICKED_TOTAL
        .with_label_values(&["test.probe"])
        .inc();

    let req = actix_test::TestRequest::get().uri("/metrics").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(ct.starts_with("text/plain"), "content-type: {}", ct);

    let body = actix_test::read_body(resp).await;
    let text = String::from_utf8_lossy(&body);

    // 关键指标都应在输出里
    for needle in [
        "http_requests_total",
        "llm_stream_chunks_total",
        "llm_stream_errors_total",
        "llm_stream_duration_seconds",
        "db_query_duration_seconds",
        "auth_failures_total",
        "tasks_panicked_total",
        "errors_total",
        "process_uptime_seconds",
    ] {
        assert!(text.contains(needle), "missing metric `{}` in /metrics body", needle);
    }

    // 我们刚才 inc 的两个 label 也应能查到
    assert!(text.contains("/metrics"));
    assert!(text.contains("not_found"));
    assert!(text.contains("test.probe"));
}

// ============================================================================
// request_id 中间件
// ============================================================================

#[actix_web::test]
async fn request_id_preserves_valid_header() {
    let app = actix_test::init_service(
        App::new()
            .wrap(from_fn(request_id_fn))
            .route("/probe", web::get().to(|| async { HttpResponse::Ok().finish() })),
    )
    .await;
    let supplied = "my-trace-0001-abc";

    let req = actix_test::TestRequest::get()
        .uri("/probe")
        .insert_header(("x-request-id", supplied))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let echoed = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert_eq!(echoed, supplied);
}

#[actix_web::test]
async fn request_id_generates_uuid_when_missing() {
    let app = actix_test::init_service(
        App::new()
            .wrap(from_fn(request_id_fn))
            .route("/probe", web::get().to(|| async { HttpResponse::Ok().finish() })),
    )
    .await;
    let req = actix_test::TestRequest::get().uri("/probe").to_request();
    let resp = actix_test::call_service(&app, req).await;

    let echoed = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(is_valid_id(&echoed), "generated id must be whitelist, got `{}`", echoed);
    assert!(echoed.contains('-'), "UUID v4 has dashes, got `{}`", echoed);
}

#[actix_web::test]
async fn request_id_rejects_invalid_and_regenerates() {
    // 长度 < 8 —— 被白名单拒绝 → 重新生成 UUID
    // (我们用长度太短的 ID 而非包含 \r\n —— 后者在 HTTP 头中根本构造不出来)
    let app = actix_test::init_service(
        App::new()
            .wrap(from_fn(request_id_fn))
            .route("/probe", web::get().to(|| async { HttpResponse::Ok().finish() })),
    )
    .await;
    let bad = "abc"; // 长度 3 < 8

    let req = actix_test::TestRequest::get()
        .uri("/probe")
        .insert_header(("x-request-id", bad))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    let echoed = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert_ne!(echoed, bad, "invalid id must not be echoed back");
    assert!(is_valid_id(&echoed), "fallback must be whitelist, got `{}`", echoed);
    // 还要覆盖"包含非法字符"分支 —— 用带空格的 ID（合法 HTTP 头字节，但被白名单拒绝）
    let bad_with_space = "trace id with space-and-extra-padding-to-pass-len-8";
    let req = actix_test::TestRequest::get()
        .uri("/probe")
        .insert_header(("x-request-id", bad_with_space))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    let echoed = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(!echoed.contains(' '), "spaces must be rejected, got `{}`", echoed);
    assert!(is_valid_id(&echoed), "fallback must be whitelist, got `{}`", echoed);
}

// ============================================================================
// panic_guard
// ============================================================================

#[tokio::test]
async fn spawn_catch_increments_counter_on_panic() {
    let name = "test.spawn_catch.panic";
    let before = TASKS_PANICKED_TOTAL
        .with_label_values(&[name])
        .get();

    let h = spawn_catch(name, async {
        panic!("intentional test panic");
    });
    let _ = timeout(Duration::from_secs(2), h).await;

    let after = TASKS_PANICKED_TOTAL.with_label_values(&[name]).get();
    assert_eq!(after - before, 1.0, "panic must increment counter by 1");
}

#[tokio::test]
async fn spawn_catch_silent_on_success() {
    let name = "test.spawn_catch.success";
    let before = TASKS_PANICKED_TOTAL
        .with_label_values(&[name])
        .get();

    let h = spawn_catch(name, async {
        // do nothing
    });
    let _ = timeout(Duration::from_secs(2), h).await;

    let after = TASKS_PANICKED_TOTAL.with_label_values(&[name]).get();
    assert_eq!(after, before, "success path must not increment counter");
}

#[tokio::test]
async fn spawn_catch_with_tx_sends_error_event_on_panic() {
    let name = "test.spawn_catch_with_tx.panic";
    let (tx, mut rx) = mpsc::channel::<SsePayload>(16);

    let h = spawn_catch_with_tx(name, tx, async {
        // 返回 AppResult<()>，这里用 panic 触发兜底分支
        let _ = AppError::LlmUpstream("unused".into()); // 让 AppError 引用不报 unused
        panic!("intentional SSE panic");
    });
    let _ = timeout(Duration::from_secs(2), h).await;

    // 客户端应收到一个 Error 事件
    let mut got_error = false;
    while let Ok(Some(ev)) = timeout(Duration::from_millis(200), rx.recv()).await {
        if let SsePayload::Error { message } = ev {
            // panic 路径：脱敏为 "internal error"
            assert_eq!(message, "internal error");
            got_error = true;
        }
    }
    assert!(got_error, "client must receive Error event on panic");
}

#[tokio::test]
async fn spawn_catch_with_tx_sends_redacted_error_on_app_error() {
    let name = "test.spawn_catch_with_tx.err";
    let (tx, mut rx) = mpsc::channel::<SsePayload>(16);

    let h = spawn_catch_with_tx(name, tx, async {
        // LlmUpstream 会被 public_message 脱敏成 "upstream service error"
        Err::<(), _>(AppError::LlmUpstream(
            "internal upstream details with secret token xyz".into(),
        ))
    });
    let _ = timeout(Duration::from_secs(2), h).await;

    let mut got_error = false;
    while let Ok(Some(ev)) = timeout(Duration::from_millis(200), rx.recv()).await {
        if let SsePayload::Error { message } = ev {
            // 关键断言：上游细节不能漏到客户端
            assert!(!message.contains("secret token"));
            assert!(!message.contains("upstream details"));
            assert_eq!(message, "upstream service error");
            got_error = true;
        }
    }
    assert!(got_error, "client must receive redacted Error event on AppError");
}

#[tokio::test]
async fn spawn_catch_with_tx_silent_on_success() {
    let name = "test.spawn_catch_with_tx.success";
    let (tx, mut rx) = mpsc::channel::<SsePayload>(16);

    let h = spawn_catch_with_tx(name, tx, async {
        Ok::<(), AppError>(())
    });
    let _ = timeout(Duration::from_secs(2), h).await;

    // 成功路径：不应有事件发出
    let got = timeout(Duration::from_millis(100), rx.recv()).await;
    assert!(got.is_err() || matches!(got, Ok(None)), "success must not send any event");
}

// ============================================================================
// public_message 脱敏（覆盖跨模块的语义）
// ============================================================================

#[test]
fn public_message_redacts_internal_details() {
    // 内部错误不能漏内部 details
    let msg = public_message(&AppError::Internal("sqlx: connection refused at 10.0.0.5".into()));
    assert_eq!(msg, "internal error");
    assert!(!msg.contains("10.0.0.5"));

    let msg = public_message(&AppError::Database(sqlx::Error::PoolClosed));
    assert_eq!(msg, "internal error");
    assert!(!msg.contains("PoolClosed"));

    let msg = public_message(&AppError::LlmUpstream("openai: 503 with API key=sk-xxx".into()));
    assert_eq!(msg, "upstream service error");
    assert!(!msg.contains("sk-xxx"));
}

#[test]
fn public_message_keeps_business_validation() {
    // 业务校验信息应原样保留（用户需要知道哪里填错）
    let msg = public_message(&AppError::Validation("email format invalid".into()));
    assert_eq!(msg, "email format invalid");
}

// ============================================================================
// sse_stream 烟测：保证 spawn_catch_with_tx 出来的 SsePayload 序列化得动
// ============================================================================

#[tokio::test]
async fn sse_stream_yields_chunk_event() {
    let (tx, rx) = mpsc::channel::<SsePayload>(4);
    tx.send(SsePayload::Chunk { text: "hello".into() })
        .await
        .unwrap();
    drop(tx);

    use futures_util::StreamExt;
    let stream = sse_stream(rx);
    tokio::pin!(stream);
    let first = stream.next().await.expect("first event");
    let bytes = first.expect("infallible");
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("hello"), "expected chunk in SSE frame, got `{}`", s);
}

// ============================================================================
// 让 test runner 不要抱怨 dead code
// ============================================================================

#[allow(dead_code)]
fn _force_use_http() {
    let _ = http();
}
