# novel-ai-rust-api

A Rust API server using Actix Web that integrates with DeepSeek and Qwen language models.

## Features
- **Actix Web** server for fast, asynchronous API handling
- **DeepSeek** model integration
- **Qwen** model integration
- **Configuration management** via environment variables
- **Health check endpoint** for monitoring
- **Prediction endpoint** for text generation

## API Endpoints

### POST /predict
Generates text using either DeepSeek or Qwen model.

**Request Body:**
```json
{
  "model": "deepseek" or "qwen",
  "prompt": "Your prompt here",
  "max_tokens": 100, (optional)
  "temperature": 0.7 (optional)
}
```

**Response:**
```json
{
  "model": "deepseek",
  "generated_text": "Generated text here",
  "tokens_used": 42
}
```

### GET /health
Returns server health status.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

## Setup

1. **Install Rust** (if not already installed): https://www.rust-lang.org/tools/install

2. **Clone the repository**

3. **Set up environment variables**:
   - Copy `.env.example` to `.env`
   - Fill in your API keys for DeepSeek and Qwen

4. **Build and run the server**:
   ```bash
   cargo run
   ```

The server will start on `http://127.0.0.1:8080`

## Dependencies
- actix-web: Web framework
- serde: Serialization/deserialization
- reqwest: HTTP client for model integration
- tokio: Async runtime
- dotenv: Environment variable management

## Observability

端到端可观测性：`tracing` 结构化日志 + Prometheus 指标 + 请求 ID 透传。

### 1. Request ID（`X-Request-Id`）

中间件自动：
- 继承上游合法 `X-Request-Id`（白名单 `[a-zA-Z0-9-]{8,64}`，防 Header/日志注入）
- 缺失/非法时生成 UUID v4
- 写入 `tracing` 子 span —— handler 内**所有**日志自动携带 `request_id` 字段
- 响应头 `X-Request-Id` 回传，方便客户端排障

```bash
$ curl -H 'X-Request-Id: trace-0001-abc' http://127.0.0.1:8080/health -i
HTTP/1.1 200 OK
x-request-id: trace-0001-abc
...
```

### 2. `/metrics` 端点（Prometheus）

`GET /metrics` 返回 Prometheus 文本格式，**无需鉴权**（仅供内网 scrape）。覆盖：

| 分类 | 指标 |
|------|------|
| HTTP | `http_requests_total{method,path,status}`、`http_request_duration_seconds`、`http_requests_in_flight` |
| LLM  | `llm_stream_chunks_total{provider,model,action}`、`llm_stream_errors_total{...}`、`llm_stream_duration_seconds` |
| DB   | `db_query_duration_seconds{op}` |
| 错误 | `errors_total{code}`、`auth_failures_total{reason}`、`tasks_panicked_total{task}` |
| 进程 | `process_uptime_seconds` |

scrape 建议（`prometheus.yml`）：

```yaml
scrape_configs:
  - job_name: novel-ai
    metrics_path: /metrics
    static_configs:
      - targets: ['127.0.0.1:8080']
```

### 3. 结构化日志

- `RUST_LOG_FORMAT=pretty` —— 人类可读，开发用
- `RUST_LOG_FORMAT=json` —— 单行 JSON，接 Loki / Elasticsearch
- `RUST_LOG=info,novel_ai_rust_api=debug,sqlx=warn` —— 精细到模块的级别

```json
{"timestamp":"...","level":"INFO","target":"...","message":"request failed","error.code":"llm_upstream_error","error.detail":"upstream llm error: openai 503","span":{"request_id":"5b2c-...","name":"request"}}
```

### 4. 错误脱敏

`AppError::error_response()` 内部：
- 完整细节（含 SQL/上游原文）→ `tracing::error!`
- 客户端响应 → `public_message()` 脱敏（`"internal error"` / `"upstream service error"` / `"authentication required"` 等）
- 同步计数 `errors_total{code}` 用于告警

业务校验/冲突错误原样保留（用户需知道哪里填错）。

