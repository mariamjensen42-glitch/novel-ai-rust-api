pub mod routes;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// 应用程序的OpenAPI规范
#[derive(OpenApi)]
#[openapi(
    components(
        schemas(
            crate::models::prediction::PredictionRequest,
            crate::models::prediction::PredictionResponse,
            crate::models::health::HealthResponse
        )
    ),
    info(
        title = "Novel AI API",
        description = "AI文本生成API服务",
        version = "0.1.0"
    )
)]
pub struct ApiDoc;

/// 配置Swagger UI路由
pub fn configure_swagger() -> SwaggerUi {
    SwaggerUi::new("/docs/{_:.*}")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
}
