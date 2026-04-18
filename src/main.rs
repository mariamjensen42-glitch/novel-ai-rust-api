use actix_web::{App, HttpServer, dev::{Service, ServiceRequest, ServiceResponse}, web};
use actix_cors::Cors;
use dotenv::dotenv;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::fmt;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use reqwest::Client;

use novel_ai_rust_api::api;
use novel_ai_rust_api::models;
use novel_ai_rust_api::config;

// 缓存配置
const CACHE_MAX_SIZE: usize = 1000;
const CACHE_TTL_SECONDS: u64 = 3600; // 1小时

// 速率限制中间件

// 速率限制存储结构
struct RateLimitStore {
    requests: HashMap<String, Vec<Instant>>,
}

impl RateLimitStore {
    fn new() -> Self {
        Self {
            requests: HashMap::new(),
        }
    }
    
    // 检查是否超过速率限制
    fn check_limit(&mut self, key: &str, max_requests: u32, window: Duration) -> bool {
        let now = Instant::now();
        
        // 获取或创建该key的请求记录
        let requests = self.requests.entry(key.to_string()).or_default();
        
        // 移除窗口外的请求
        requests.retain(|&time| now.duration_since(time) < window);
        
        // 检查是否超过限制
        if requests.len() >= max_requests as usize {
            false
        } else {
            // 添加当前请求
            requests.push(now);
            true
        }
    }
}

// 速率限制中间件
struct RateLimiter {
    store: Arc<Mutex<RateLimitStore>>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(RateLimitStore::new())),
        }
    }
}

// 实现Transform trait
impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = RateLimitMiddleware<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RateLimitMiddleware {
            service: Arc::new(service),
            store: self.store.clone(),
        }))
    }
}

// 中间件实现
struct RateLimitMiddleware<S> {
    service: Arc<S>,
    store: Arc<Mutex<RateLimitStore>>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.as_ref().poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // 获取客户端IP
        let client_ip = req.connection_info().realip_remote_addr().unwrap_or("0.0.0.0").to_string();
        let store = self.store.clone();
        let service = Arc::clone(&self.service);
        
        Box::pin(async move {
            // 检查速率限制
            let mut store = store.lock().unwrap();
            if !store.check_limit(&client_ip, 60, Duration::from_secs(60)) {
                // 超过限制，返回429错误
                Err(actix_web::error::ErrorTooManyRequests("Rate limit exceeded"))
            } else {
                // 未超过限制，继续处理请求
                service.call(req).await
            }
        })
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    // 初始化tracing订阅者
    fmt::init();
    
    // 创建HTTP客户端实例（带连接池）
    let http_client = Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .expect("Failed to create HTTP client");
    let shared_http_client = Arc::new(http_client);
    
    // 创建缓存实例
    let cache = models::cache::PredictionCache::new(CACHE_MAX_SIZE, CACHE_TTL_SECONDS);
    let shared_cache = models::cache::SharedCache::new(Mutex::new(cache));
    
    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(
                Cors::permissive()
            )
            .wrap(RateLimiter::new())
            .app_data(web::Data::new(shared_cache.clone()))
            .app_data(web::Data::new(shared_http_client.clone()))
            .service(api::routes::predict)
            .service(api::routes::health_check)
            .service(api::configure_swagger())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
