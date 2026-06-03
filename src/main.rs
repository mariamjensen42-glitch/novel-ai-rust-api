use std::sync::Arc;
use std::time::Duration;

use actix_cors::Cors;
use actix_web::middleware::DefaultHeaders;
use actix_web::{web, App, HttpResponse, HttpServer};
use reqwest::Client;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::EnvFilter;
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
use novel_ai_rust_api::models::character::{Character, CreateCharacterRequest, UpdateCharacterRequest};
use novel_ai_rust_api::models::chapter::{Chapter, CreateChapterRequest, ReorderRequest, UpdateChapterRequest};
use novel_ai_rust_api::models::novel::{CreateNovelRequest, Novel, UpdateNovelRequest};
use novel_ai_rust_api::models::outline::{CreateOutlineNodeRequest, OutlineNode, OutlineTreeNode, UpdateOutlineNodeRequest};
use novel_ai_rust_api::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};
use novel_ai_rust_api::models::user::{AuthResponse, LoginRequest, RegisterRequest, User};

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
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init();
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
