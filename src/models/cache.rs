use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::models::prediction::{PredictionRequest, PredictionResponse};

#[derive(Debug, Clone)]
pub struct CacheEntry {
    response: PredictionResponse,
    timestamp: Instant,
}

pub struct PredictionCache {
    store: HashMap<String, CacheEntry>,
    max_size: usize,
    ttl: Duration,
}

impl PredictionCache {
    pub fn new(max_size: usize, ttl_seconds: u64) -> Self {
        Self {
            store: HashMap::new(),
            max_size,
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<PredictionResponse> {
        // 清理过期项
        self.cleanup();

        // 检查缓存中是否存在该键
        if let Some(entry) = self.store.get(key) {
            // 检查是否过期
            if Instant::now().duration_since(entry.timestamp) < self.ttl {
                return Some(entry.response.clone());
            } else {
                // 过期则移除
                self.store.remove(key);
            }
        }
        None
    }

    pub fn set(&mut self, key: String, response: PredictionResponse) {
        // 清理过期项
        self.cleanup();

        // 如果缓存达到最大大小，移除最旧的项
        if self.store.len() >= self.max_size {
            if let Some(oldest_key) = self.get_oldest_key() {
                self.store.remove(&oldest_key);
            }
        }

        // 添加新项
        self.store.insert(key, CacheEntry {
            response,
            timestamp: Instant::now(),
        });
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        self.store.retain(|_, entry| {
            now.duration_since(entry.timestamp) < self.ttl
        });
    }

    fn get_oldest_key(&self) -> Option<String> {
        self.store.iter()
            .min_by(|(_, a), (_, b)| a.timestamp.cmp(&b.timestamp))
            .map(|(key, _)| key.clone())
    }
}

// 用于创建线程安全的缓存实例
pub type SharedCache = Arc<Mutex<PredictionCache>>;

// 生成缓存键的函数
pub fn generate_cache_key(request: &PredictionRequest) -> String {
    format!("{}_{}_{}_{}", 
        request.model, 
        request.prompt, 
        request.max_tokens.unwrap_or(100), 
        request.temperature.unwrap_or(0.7)
    )
}

#[cfg(test)]
mod tests {
    use super::{PredictionCache, generate_cache_key};
    use super::super::prediction::{PredictionRequest, PredictionResponse};
    use std::time::Duration;

    #[test]
    fn test_cache_new() {
        let cache = PredictionCache::new(100, 3600);
        assert_eq!(cache.store.is_empty(), true);
    }

    #[test]
    fn test_cache_get_set() {
        let mut cache = PredictionCache::new(100, 3600);
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        let response = PredictionResponse {
            model: "deepseek".to_string(),
            generated_text: "test response".to_string(),
            tokens_used: 10,
        };
        let key = generate_cache_key(&request);
        cache.set(key.clone(), response.clone());
        let cached_response = cache.get(&key);
        assert_eq!(cached_response, Some(response));
    }

    #[test]
    fn test_cache_expiration() {
        let mut cache = PredictionCache::new(100, 1);
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        let response = PredictionResponse {
            model: "deepseek".to_string(),
            generated_text: "test response".to_string(),
            tokens_used: 10,
        };
        let key = generate_cache_key(&request);
        cache.set(key.clone(), response);
        std::thread::sleep(Duration::from_secs(2));
        let cached_response = cache.get(&key);
        assert_eq!(cached_response, None);
    }

    #[test]
    fn test_cache_max_size() {
        let mut cache = PredictionCache::new(2, 3600);
        for i in 0..3 {
            let request = PredictionRequest {
                model: "deepseek".to_string(),
                prompt: format!("test prompt {}", i),
                max_tokens: Some(100),
                temperature: Some(0.7),
            };
            let response = PredictionResponse {
                model: "deepseek".to_string(),
                generated_text: format!("test response {}", i),
                tokens_used: 10,
            };
            let key = generate_cache_key(&request);
            cache.set(key, response);
        }
        assert_eq!(cache.store.len(), 2);
    }

    #[test]
    fn test_generate_cache_key() {
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        let key = generate_cache_key(&request);
        assert_eq!(key, "deepseek_test prompt_100_0.7");
    }

    #[test]
    fn test_generate_cache_key_with_defaults() {
        let request = PredictionRequest {
            model: "qwen".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: None,
            temperature: None,
        };
        let key = generate_cache_key(&request);
        assert_eq!(key, "qwen_test prompt_100_0.7");
    }
}
