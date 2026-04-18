use std::env;

pub struct Config {
    pub deepseek_api_key: String,
    pub deepseek_endpoint: String,
    pub qwen_api_key: String,
    pub qwen_endpoint: String,
}

pub fn get_config() -> Config {
    Config {
        deepseek_api_key: env::var("DEEPSEEK_API_KEY").unwrap_or_else(|_| "".to_string()),
        deepseek_endpoint: env::var("DEEPSEEK_ENDPOINT").unwrap_or_else(|_| "https://api.deepseek.com/v1/completions".to_string()),
        qwen_api_key: env::var("QWEN_API_KEY").unwrap_or_else(|_| "".to_string()),
        qwen_endpoint: env::var("QWEN_ENDPOINT").unwrap_or_else(|_| "https://api.qwen.com/v1/completions".to_string()),
    }
}
