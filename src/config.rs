use std::env;
use std::sync::OnceLock;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub endpoint: String,
    pub default_model: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl_hours: i64,
    pub providers: Vec<ProviderConfig>,
    pub rate_limit_per_min: u32,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(load)
}

fn load() -> Config {
    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://./data/novel.db".to_string());
    let jwt_secret =
        env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-production-please".to_string());
    let jwt_ttl_hours = env::var("JWT_TTL_HOURS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(72);
    let rate_limit_per_min = env::var("RATE_LIMIT_PER_MIN")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    let mut providers = Vec::new();
    if let Ok(api_key) = env::var("DEEPSEEK_API_KEY") {
        if !api_key.is_empty() {
            providers.push(ProviderConfig {
                name: "deepseek".to_string(),
                api_key,
                endpoint: env::var("DEEPSEEK_ENDPOINT").unwrap_or_else(|_| {
                    "https://api.deepseek.com/v1/chat/completions".to_string()
                }),
                default_model: env::var("DEEPSEEK_MODEL")
                    .unwrap_or_else(|_| "deepseek-chat".to_string()),
            });
        }
    }
    if let Ok(api_key) = env::var("QWEN_API_KEY") {
        if !api_key.is_empty() {
            providers.push(ProviderConfig {
                name: "qwen".to_string(),
                api_key,
                endpoint: env::var("QWEN_ENDPOINT").unwrap_or_else(|_| {
                    "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
                        .to_string()
                }),
                default_model: env::var("QWEN_MODEL").unwrap_or_else(|_| "qwen-plus".to_string()),
            });
        }
    }

    Config {
        bind_addr,
        database_url,
        jwt_secret,
        jwt_ttl_hours,
        providers,
        rate_limit_per_min,
    }
}

pub fn find_provider(name: &str) -> Option<&'static ProviderConfig> {
    get_config().providers.iter().find(|p| p.name == name)
}
