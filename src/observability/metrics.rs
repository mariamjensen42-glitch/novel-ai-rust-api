//! Prometheus 指标注册表与命名常量
//!
//! 所有指标全局 `Lazy` 初始化，注册到全局 [`REGISTRY`]。
//! HTTP/LLM/DB 三大类覆盖，由业务调用方主动 `with_label_values().inc()` 触发。

use once_cell::sync::Lazy;
use prometheus::{
    Counter, CounterVec, Encoder, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, Opts,
    Registry, TextEncoder,
};

/// 全局 Prometheus 注册表
pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

/// 进程启动时间戳（秒），用于 `process_uptime_seconds` 计算
pub static START_TIME: Lazy<std::time::Instant> = Lazy::new(std::time::Instant::now);

// ============================================================================
// HTTP
// ============================================================================

/// HTTP 请求总数（按 method/path/status 维度）
pub static HTTP_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    let v = CounterVec::new(
        Opts::new("http_requests_total", "Total HTTP requests"),
        &["method", "path", "status"],
    )
    .expect("http_requests_total");
    REGISTRY.register(Box::new(v.clone())).expect("register http_requests_total");
    v
});

/// HTTP 请求延迟直方图（秒）
pub static HTTP_REQUEST_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    let v = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request latency").buckets(
            vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
        ),
        &["method", "path"],
    )
    .expect("http_request_duration_seconds");
    REGISTRY
        .register(Box::new(v.clone()))
        .expect("register http_request_duration_seconds");
    v
});

/// HTTP 正在处理的请求数（按 path）
pub static HTTP_REQUESTS_IN_FLIGHT: Lazy<GaugeVec> = Lazy::new(|| {
    let v = GaugeVec::new(
        Opts::new("http_requests_in_flight", "In-flight HTTP requests"),
        &["path"],
    )
    .expect("http_requests_in_flight");
    REGISTRY
        .register(Box::new(v.clone()))
        .expect("register http_requests_in_flight");
    v
});

// ============================================================================
// LLM
// ============================================================================

/// LLM 流式 chunk 计数
pub static LLM_STREAM_CHUNKS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    let v = CounterVec::new(
        Opts::new("llm_stream_chunks_total", "LLM streamed chunks"),
        &["provider", "model", "action"],
    )
    .expect("llm_stream_chunks_total");
    REGISTRY
        .register(Box::new(v.clone()))
        .expect("register llm_stream_chunks_total");
    v
});

/// LLM 流式错误计数（按 kind 区分）
pub static LLM_STREAM_ERRORS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    let v = CounterVec::new(
        Opts::new("llm_stream_errors_total", "LLM stream errors by kind"),
        &["provider", "model", "action", "kind"],
    )
    .expect("llm_stream_errors_total");
    REGISTRY
        .register(Box::new(v.clone()))
        .expect("register llm_stream_errors_total");
    v
});

/// LLM 完整生成耗时直方图（秒）
pub static LLM_STREAM_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    let v = HistogramVec::new(
        HistogramOpts::new("llm_stream_duration_seconds", "LLM full generation duration")
            .buckets(vec![0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0]),
        &["provider", "model", "action"],
    )
    .expect("llm_stream_duration_seconds");
    REGISTRY
        .register(Box::new(v.clone()))
        .expect("register llm_stream_duration_seconds");
    v
});

// ============================================================================
// DB
// ============================================================================

/// DB 查询耗时直方图（秒，按 op 区分）
pub static DB_QUERY_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    let v = HistogramVec::new(
        HistogramOpts::new("db_query_duration_seconds", "DB query latency")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
        &["op"],
    )
    .expect("db_query_duration_seconds");
    REGISTRY
        .register(Box::new(v.clone()))
        .expect("register db_query_duration_seconds");
    v
});

// ============================================================================
// 鉴权 / 错误 / Panic
// ============================================================================

/// 鉴权失败次数（按 reason）
pub static AUTH_FAILURES_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    let v = CounterVec::new(
        Opts::new("auth_failures_total", "Auth failures by reason"),
        &["reason"],
    )
    .expect("auth_failures_total");
    REGISTRY
        .register(Box::new(v.clone()))
        .expect("register auth_failures_total");
    v
});

/// 后台任务 panic 次数
pub static TASKS_PANICKED_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    let v = CounterVec::new(
        Opts::new("tasks_panicked_total", "Background task panics"),
        &["task"],
    )
    .expect("tasks_panicked_total");
    REGISTRY
        .register(Box::new(v.clone()))
        .expect("register tasks_panicked_total");
    v
});

/// 错误码频次
pub static ERRORS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    let v = CounterVec::new(Opts::new("errors_total", "Errors by code"), &["code"])
        .expect("errors_total");
    REGISTRY
        .register(Box::new(v.clone()))
        .expect("register errors_total");
    v
});

// ============================================================================
// 进程级
// ============================================================================

/// 进程启动时长（秒），由 [`record_uptime`] 周期性更新
pub static PROCESS_UPTIME_SECONDS: Lazy<Gauge> = Lazy::new(|| {
    let g = Gauge::new("process_uptime_seconds", "Process uptime in seconds")
        .expect("process_uptime_seconds");
    REGISTRY
        .register(Box::new(g.clone()))
        .expect("register process_uptime_seconds");
    g
});

/// 刷新 `process_uptime_seconds`
pub fn record_uptime() {
    PROCESS_UPTIME_SECONDS.set(START_TIME.elapsed().as_secs_f64());
}

// ============================================================================
// 端点渲染
// ============================================================================

/// 渲染 Prometheus 文本格式输出（给 `/metrics` 端点用）
pub fn render_text() -> Vec<u8> {
    record_uptime();
    let encoder = TextEncoder::new();
    let mf = REGISTRY.gather();
    let mut buf = Vec::new();
    let _ = encoder.encode(&mf, &mut buf);
    buf
}
