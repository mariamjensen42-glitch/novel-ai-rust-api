# AI 写小说后端 API — 设计文档

**日期**：2026-06-02
**目标**：将现有 `novel-ai-rust-api`（一个通用 LLM API 代理）重构为一个面向"AI 辅助长篇小说创作"的完整后端工作台。

---

## 1. 背景与目标

### 1.1 现状
- Actix Web 4 + reqwest 调上游 DeepSeek / Qwen
- 仅有两个端点：`/predict`（通用 prompt 补全）和 `/health`
- 无持久化、无用户、无业务对象

### 1.2 目标
提供一个支持多用户、可管理 Project / Novel / Chapter / Character / Outline 的创作后端，并对外暴露 6+ 种 AI 写作动作（续写、改写、扩写、摘要、对话、大纲、人物设计），全部以 SSE 流式返回。

### 1.3 非目标（YAGNI）
- 不做实时多人协作
- 不存历史版本
- 不做支付/订阅
- 不做 Web 前端（仅 API）
- 不做复杂权限/角色系统（仅 owner 校验）

---

## 2. 关键决策（已与用户对齐）

| # | 决策 | 选择 |
|---|------|------|
| 1 | 功能范围 | 完整创作工作台（Project/Novel/Chapter/Character/Outline） |
| 2 | 多用户/认证 | 多用户 + JWT |
| 3 | 生成返回方式 | SSE 流式 |
| 4 | 生成动作集 | 多动作丰富（continue/rewrite/expand/summarize/dialogue/outline/character） |
| 5 | 作品层级 | 三级 Project / Novel / Chapter |
| 6 | LLM 抽象 | Provider trait + DeepseekProvider / QwenProvider |
| 7 | 持久化 | SQLite + sqlx |
| 8 | 架构 | 分层单体（handler → service → repository → provider） |

---

## 3. 总体架构

### 3.1 依赖方向

```
handlers → services → repositories → sqlx
                    ↘
                      providers (LLM)
                    ↘
                      prompts

auth 中间件横切注入 current_user
```

严格单向：handler 不直接调 repository，repository 不感知 HTTP 类型。

### 3.2 目录结构

```
src/
├── main.rs                     # 启动、装配中间件、初始化 DB
├── lib.rs                      # 模块声明
├── config.rs                   # 环境变量 + LLM Provider 配置
├── db/
│   ├── mod.rs
│   ├── pool.rs                 # sqlx::SqlitePool 单例
│   └── migrations/             # 内联 SQL 迁移
├── error.rs                    # AppError + ResponseError
├── auth/
│   ├── mod.rs
│   ├── jwt.rs                  # encode/decode
│   ├── password.rs             # argon2 哈希
│   └── middleware.rs           # Bearer 提取 + 注入 current_user
├── models/                     # 领域类型（DTO + DB row）
│   ├── mod.rs
│   ├── user.rs
│   ├── project.rs
│   ├── novel.rs
│   ├── chapter.rs
│   ├── character.rs
│   └── outline.rs
├── repositories/               # sqlx 数据访问
│   ├── mod.rs
│   ├── users.rs
│   ├── projects.rs
│   ├── novels.rs
│   ├── chapters.rs
│   ├── characters.rs
│   ├── chapter_characters.rs   # 章节-人物 关联
│   └── outlines.rs
├── services/                   # 业务逻辑
│   ├── mod.rs
│   ├── auth_service.rs
│   ├── project_service.rs
│   ├── novel_service.rs
│   ├── chapter_service.rs
│   ├── character_service.rs
│   ├── outline_service.rs
│   └── generation_service.rs
├── providers/                  # LLM 抽象
│   ├── mod.rs                  # LlmProvider trait
│   ├── registry.rs
│   ├── deepseek.rs
│   └── qwen.rs
├── prompts/                    # 模板与组装
│   ├── mod.rs
│   ├── continue.rs
│   ├── rewrite.rs
│   ├── expand.rs
│   ├── summarize.rs
│   ├── dialogue.rs
│   ├── outline.rs
│   └── character.rs
├── handlers/                   # HTTP/SSE 端点
│   ├── mod.rs
│   ├── auth.rs
│   ├── projects.rs
│   ├── novels.rs
│   ├── chapters.rs
│   ├── characters.rs
│   ├── outlines.rs
│   ├── generation.rs
│   └── health.rs
└── sse.rs                      # SSE 响应工具
```

---

## 4. 数据模型（SQLite）

### 4.1 表结构

```sql
-- 1. 用户
CREATE TABLE users (
  id            TEXT PRIMARY KEY,
  email         TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  display_name  TEXT NOT NULL,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL
);

-- 2. 项目
CREATE TABLE projects (
  id          TEXT PRIMARY KEY,
  owner_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
CREATE INDEX idx_projects_owner ON projects(owner_id);

-- 3. 小说
CREATE TABLE novels (
  id          TEXT PRIMARY KEY,
  project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title       TEXT NOT NULL,
  synopsis    TEXT NOT NULL DEFAULT '',
  genre       TEXT NOT NULL DEFAULT '',
  style       TEXT NOT NULL DEFAULT '',
  pov         TEXT NOT NULL DEFAULT 'third',
  tone        TEXT NOT NULL DEFAULT '',
  target_word_count INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
CREATE INDEX idx_novels_project ON novels(project_id);

-- 4. 章节
CREATE TABLE chapters (
  id           TEXT PRIMARY KEY,
  novel_id     TEXT NOT NULL REFERENCES novels(id) ON DELETE CASCADE,
  title        TEXT NOT NULL,
  summary      TEXT NOT NULL DEFAULT '',
  content      TEXT NOT NULL DEFAULT '',
  order_index  INTEGER NOT NULL,
  status       TEXT NOT NULL DEFAULT 'draft',
  word_count   INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  UNIQUE(novel_id, order_index)
);
CREATE INDEX idx_chapters_novel ON chapters(novel_id);

-- 5. 人物
CREATE TABLE characters (
  id          TEXT PRIMARY KEY,
  novel_id    TEXT NOT NULL REFERENCES novels(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,
  role        TEXT NOT NULL DEFAULT 'supporting',
  description TEXT NOT NULL DEFAULT '',
  traits      TEXT NOT NULL DEFAULT '',
  backstory   TEXT NOT NULL DEFAULT '',
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
CREATE INDEX idx_characters_novel ON characters(novel_id);

-- 6. 大纲节点（树形）
CREATE TABLE outline_nodes (
  id          TEXT PRIMARY KEY,
  novel_id    TEXT NOT NULL REFERENCES novels(id) ON DELETE CASCADE,
  parent_id   TEXT REFERENCES outline_nodes(id) ON DELETE CASCADE,
  title       TEXT NOT NULL,
  summary     TEXT NOT NULL DEFAULT '',
  order_index INTEGER NOT NULL,
  chapter_id  TEXT REFERENCES chapters(id) ON DELETE SET NULL,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
CREATE INDEX idx_outline_novel ON outline_nodes(novel_id);
CREATE INDEX idx_outline_parent ON outline_nodes(parent_id);

-- 7. 章节-人物 关联
CREATE TABLE chapter_characters (
  chapter_id   TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
  character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  PRIMARY KEY (chapter_id, character_id)
);
```

### 4.2 设计要点
- **id 全部 TEXT（nanoid）**：避免暴露数量、便于将来扩展
- **时间戳 RFC3339 字符串**：可读、避免时区坑
- **ON DELETE CASCADE**：删上层自动清下层
- **大纲用闭包表/树形**：可"展开某节点继续写"，节点可挂一个章节
- **content 直接 TEXT**：单字段最大 1GB
- **不存生成历史**：YAGNI

---

## 5. API 端点

### 5.1 公共端点（无需 JWT）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/auth/register` | 邮箱+密码+昵称 → JWT |
| POST | `/auth/login` | 邮箱+密码 → JWT |
| GET  | `/health` | 服务健康 + Provider 状态 |
| GET  | `/swagger-ui/` | OpenAPI 文档 |

### 5.2 资源端点（需 JWT + 所有权校验）

| 方法 | 路径 |
|------|------|
| GET/POST | `/projects` |
| GET/PATCH/DELETE | `/projects/{id}` |
| GET/POST | `/projects/{project_id}/novels` |
| GET/PATCH/DELETE | `/novels/{id}` |
| GET/POST | `/novels/{novel_id}/chapters` |
| GET/PATCH/DELETE | `/chapters/{id}` |
| POST | `/chapters/{id}/reorder` |
| GET/POST | `/novels/{novel_id}/characters` |
| GET/PATCH/DELETE | `/characters/{id}` |
| GET | `/novels/{novel_id}/outline` |
| POST | `/novels/{novel_id}/outline/nodes` |
| PATCH/DELETE | `/outline/nodes/{id}` |

### 5.3 生成端点（SSE）

请求：JSON；响应：`text/event-stream`。

事件：
- `chunk` — `{ "text": "..." }`
- `usage` — `{ "prompt_tokens": N, "completion_tokens": N }`
- `done` — `{ "chapter_id": "...", "new_word_count": N }`
- `error` — `{ "message": "..." }`

| 路径 | 关键参数 | 作用 |
|------|----------|------|
| `/generation/continue` | `chapter_id, model, target_words?, temperature?` | 续写 |
| `/generation/rewrite` | `chapter_id, model, instruction, range?` | 改写 |
| `/generation/expand` | `chapter_id, model, anchor, target_words?` | 锚点扩写 |
| `/generation/summarize` | `chapter_id, model, max_words?` | 章节摘要 |
| `/generation/dialogue` | `chapter_id, model, character_ids: string[], situation` | 对话（character_ids 是 Character.id） |
| `/generation/outline` | `novel_id, model, idea, depth?` | 大纲生成 |
| `/generation/character` | `novel_id, model, name?, concept, role?` | 人物设计 |

### 5.4 续写交互流程

```
POST /generation/continue
  → auth 中间件解析 JWT, 注入 current_user
  → handler 校验 chapter 归属 current_user
  → prompt::continue::build 收集 chapter/novel/characters/outline
  → provider.stream(prompt) -> SseStream
  → handler 把 chunk 推 SSE 给 client
  → provider done, handler 写回 chapters.content
  → handler 推 'done'
```

---

## 6. 错误处理

### 6.1 AppError

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("validation error: {0}")]  Validation(String),
    #[error("unauthorized")]           Unauthorized,
    #[error("forbidden")]              Forbidden,
    #[error("not found: {0}")]          NotFound(String),
    #[error("conflict: {0}")]           Conflict(String),
    #[error("upstream llm error: {0}")] LlmUpstream(String),
    #[error("database error: {0}")]     Database(#[from] sqlx::Error),
    #[error("internal error: {0}")]     Internal(String),
}
```

### 6.2 HTTP 映射
| 变体 | 状态码 |
|------|--------|
| Validation | 400 |
| Unauthorized | 401 |
| Forbidden | 403 |
| NotFound | 404 |
| Conflict | 409 |
| LlmUpstream | 502 |
| Database / Internal | 500 |

### 6.3 响应体
```json
{ "error": { "code": "validation_error", "message": "..." } }
```

### 6.4 SSE 错误
流中途出错 → 发 `event: error` 事件，正常关闭（不抛 5xx）。

---

## 7. 配置

```rust
pub struct Config {
    pub server: ServerConfig,        // BIND_ADDR, WORKERS
    pub db: DbConfig,                // DATABASE_URL
    pub auth: AuthConfig,            // JWT_SECRET, JWT_TTL_HOURS
    pub providers: Vec<ProviderCfg>, // name/kind/api_key/endpoint/default_model
    pub rate_limit: RateLimitCfg,    // REQUESTS_PER_MIN
}
```

环境变量：
```
BIND_ADDR=127.0.0.1:8080
DATABASE_URL=sqlite://./data/novel.db
JWT_SECRET=change-me
JWT_TTL_HOURS=72
DEEPSEEK_API_KEY=...
DEEPSEEK_ENDPOINT=https://api.deepseek.com/v1/chat/completions
DEEPSEEK_MODEL=deepseek-chat
QWEN_API_KEY=...
QWEN_ENDPOINT=https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions
QWEN_MODEL=qwen-plus
RATE_LIMIT_PER_MIN=60
```

`Config` 用 `once_cell::sync::Lazy<Config>` 启动时加载并校验，缺关键值立即 panic + 清晰日志。

---

## 8. Provider 抽象

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn stream(
        &self,
        req: CompletionRequest,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<Usage, AppError>;
}
```

- Deepseek / Qwen 都走 OpenAI 兼容 chat completions 流式协议
- 差异仅在 `base_url` + 鉴权头
- `ProviderRegistry` 根据字符串返回实例
- 加新模型：实现 trait + registry 注册一行

---

## 9. Prompt 模板

- 每个动作一个文件，签名 `fn build_xxx(ctx) -> ChatMessages`
- `ChatMessages { system: String, user: String }`
- 不再 f-string 手拼，便于将来切换协议
- 自动收集上下文：novel 设定 / 前 N 章摘要 / 相关 characters / 当前 outline 节点

---

## 10. 中间件

- **Auth**：Bearer JWT → 注入 `current_user` 到请求扩展
- **RateLimit**：基于 user_id，未登录用 IP，沿用现有 60 req/min 滑动窗口实现
- **CORS**：`permissive()`（开发友好）
- **Tracing**：`TracingLogger`，每个 generation 端点 span 含 `user_id / action / chapter_id`

---

## 11. 测试策略

| 层级 | 范围 | 工具 |
|------|------|------|
| 单元 | models 校验 | `cargo test` |
| 单元 | repositories（内存 sqlite） | `sqlx` |
| 单元 | prompts 输出快照 | `insta`（可选） |
| 单元 | providers 行为 | `wiremock` 模拟 HTTP |
| 集成 | handlers 端到端（mock provider） | actix `TestServer` |
| E2E 手工 | 完整流程 | 真实 LLM key |

覆盖目标：service ≥ 70%，handler 集成路径全覆盖。

---

## 12. 依赖调整

**新增**：
- `sqlx` (sqlite, runtime-tokio-rustls, macros, chrono, migrate)
- `tokio-stream`, `futures-util`
- `argon2`, `password-hash`, `rand`
- `jsonwebtoken`
- `nanoid`
- `thiserror`
- `async-trait`
- `wiremock` (dev)
- `once_cell`
- `chrono` (id 与时间戳)

**移除/降级**：
- `redis`（未用）
- `utoipa-redoc`（保留 utoipa + swagger-ui 即可）
- `actix-ratelimit`（改用项目内中间件）
- `actix-web-actors`（暂不需要）

---

## 13. 端到端用户故事（验收）

1. 用户 A 注册 → 登录拿到 JWT
2. 创建一个 Project "奇幻大陆"
3. 在项目下创建 Novel "龙之血脉"，设置 genre/style/pov
4. 创建 3 个 Characters（主角/导师/反派）
5. 创建大纲：卷一 → 章1 → 场景1
6. 创建空 Chapter 1，把大纲节点挂上去
7. `POST /generation/continue` → SSE 流式返回正文 → 自动写入 chapter.content
8. `POST /generation/summarize` → 拿到 summary 写回 chapter.summary
9. `POST /generation/character` → 新建一个人物
10. 重复 6-9，创作更多章节

---

## 14. 范围与时间盒

本次重构预计一次完成（约 1-2 个实施 session），后续可增量扩展：
- 章节历史版本（`chapter_revisions` 表）
- 协作/分享（链接 token）
- 高级检索/全文搜索
- Web 前端
