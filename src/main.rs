use std::sync::Arc;
use std::time::Duration;

use actix_cors::Cors;
use actix_web::middleware::DefaultHeaders;
use actix_web::{web, App, HttpResponse, HttpServer};
use reqwest::Client;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use novel_ai_rust_api::auth::AuthMiddleware;
use novel_ai_rust_api::config::get_config;
use novel_ai_rust_api::db;
use novel_ai_rust_api::handlers;
use novel_ai_rust_api::handlers::generation::{
    CharacterGenRequest, ConsistencyCheckRequest, ContinueRequest, DialogueRequest, ExpandRequest,
    OutlineGenRequest, PolishRequest, RewriteRequest, StyleTransferRequest, SummarizeRequest,
    TranslateRequest,
};
use novel_ai_rust_api::middleware::request_id::middleware as request_id_fn;
use novel_ai_rust_api::models::character::{Character, CreateCharacterRequest, UpdateCharacterRequest};
use novel_ai_rust_api::models::chapter::{Chapter, CreateChapterRequest, ReorderRequest, UpdateChapterRequest};
use novel_ai_rust_api::models::novel::{CreateNovelRequest, Novel, UpdateNovelRequest};
use novel_ai_rust_api::models::outline::{CreateOutlineNodeRequest, OutlineNode, OutlineTreeNode, UpdateOutlineNodeRequest};
use novel_ai_rust_api::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};
use novel_ai_rust_api::models::user::{AuthResponse, LoginRequest, RegisterRequest, User};
use novel_ai_rust_api::observability::metrics;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "AI Novel Writing API",
        version = "0.2.0",
        description = "AI 辅助长篇小说创作后端",
    ),
    components(
        schemas(
            User, RegisterRequest, LoginRequest, AuthResponse,
            Project, CreateProjectRequest, UpdateProjectRequest,
            Novel, CreateNovelRequest, UpdateNovelRequest,
            Chapter, CreateChapterRequest, UpdateChapterRequest, ReorderRequest,
            Character, CreateCharacterRequest, UpdateCharacterRequest,
            OutlineNode, OutlineTreeNode, CreateOutlineNodeRequest, UpdateOutlineNodeRequest,
            ContinueRequest, RewriteRequest, ExpandRequest, SummarizeRequest, DialogueRequest,
            OutlineGenRequest, CharacterGenRequest,
            TranslateRequest, PolishRequest, StyleTransferRequest, ConsistencyCheckRequest,
        )
    ),
    tags(
        (name = "auth", description = "认证"),
        (name = "projects", description = "项目"),
        (name = "novels", description = "小说"),
        (name = "chapters", description = "章节"),
        (name = "characters", description = "人物"),
        (name = "outline", description = "大纲"),
        (name = "generation", description = "AI 生成（续写 / 改写 / 扩写 / 摘要 / 对话 / 大纲 / 人物 / 翻译 / 润色 / 风格转换 / 人设一致性）"),
        (name = "health", description = "健康检查"),
    )
)]
struct ApiDoc;

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // `RUST_LOG_FORMAT=json` 输出 JSON 行（接 Loki/ES），否则 pretty（dev 友好）
    let json_mode = std::env::var("RUST_LOG_FORMAT").as_deref() == Ok("json");

    let result = if json_mode {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(false)
                    .with_target(true),
            )
            .try_init()
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty().with_target(true))
            .try_init()
    };

    let _ = result;
}

fn build_http_client() -> Arc<Client> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build http client");
    Arc::new(client)
}

async fn serve_openapi() -> HttpResponse {
    HttpResponse::Ok().json(ApiDoc::openapi())
}

async fn metrics_endpoint() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(metrics::render_text())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _ = dotenvy::dotenv();
    init_tracing();

    let pool = db::pool::init_pool()
        .await
        .expect("failed to initialize database");
    db::pool::set_pool(pool);

    let cfg = get_config();
    let bind = cfg.bind_addr.clone();
    let http_client = build_http_client();

    tracing::info!("Starting AI Novel Writing API on {}", bind);

    HttpServer::new(move || {
        let openapi = ApiDoc::openapi();
        App::new()
            .wrap(TracingLogger::default())
            .wrap(actix_web::middleware::from_fn(request_id_fn)) // 注入 request_id 到 span + extensions + 响应头
            .wrap(AuthMiddleware)
            .wrap(DefaultHeaders::new().add(("X-Service", "novel-ai")))
            .wrap(Cors::permissive())
            .app_data(web::Data::new(http_client.clone()))
            .app_data(web::JsonConfig::default().limit(1024 * 1024))
            .service(
                SwaggerUi::new("/docs/{_:.*}")
                    .url("/api-docs/openapi.json", openapi),
            )
            .route("/api-docs/openapi.json", web::get().to(serve_openapi))
            .route("/metrics", web::get().to(metrics_endpoint))
            .configure(handlers::health::configure)
            .configure(handlers::auth::configure)
            .configure(handlers::projects::configure)
            .configure(handlers::novels::configure)
            .configure(handlers::chapters::configure)
            .configure(handlers::characters::configure)
            .configure(handlers::outlines::configure)
            .configure(handlers::generation::configure)
    })
    .bind(&bind)?
    .run()
    .await
}
