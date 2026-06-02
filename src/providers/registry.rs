use std::collections::HashMap;
use std::sync::Arc;

use reqwest::Client;

use crate::config::{find_provider, ProviderConfig};
use crate::error::{AppError, AppResult};
use crate::providers::openai_compat::OpenAiCompatibleProvider;
use crate::providers::LlmProvider;

pub fn build_provider(name: &str, client: Arc<Client>) -> AppResult<Box<dyn LlmProvider>> {
    let cfg = find_provider(name)
        .ok_or_else(|| AppError::Validation(format!("unknown model provider: {}", name)))?;
    Ok(Box::new(OpenAiCompatibleProvider::new(
        name_for(name),
        cfg.api_key.clone(),
        cfg.endpoint.clone(),
        client,
    )))
}

pub fn default_model(name: &str) -> Option<&'static str> {
    let _ = name;
    None
}

pub fn available_providers() -> Vec<&'static str> {
    let cfg = crate::config::get_config();
    cfg.providers.iter().map(|p| p.name.as_str()).collect()
}

pub fn get_provider_config(name: &str) -> Option<&'static ProviderConfig> {
    find_provider(name)
}

fn name_for(name: &str) -> &'static str {
    match name {
        "deepseek" => "deepseek",
        "qwen" => "qwen",
        other => {
            let s: &'static str = Box::leak(other.to_string().into_boxed_str());
            s
        }
    }
}

pub fn _unused_map() -> HashMap<String, String> {
    HashMap::new()
}
