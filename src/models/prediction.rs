use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 预测请求模型
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct PredictionRequest {
    /// 模型名称，支持 "deepseek" 或 "qwen"
    #[schema(example = "deepseek", pattern = "^(deepseek|qwen)$")]
    pub model: String,
    /// 提示文本
    #[schema(example = "Write a short story about AI", max_length = 10000)]
    pub prompt: String,
    /// 最大生成token数，默认值为256，范围1-2048
    #[schema(example = 256, minimum = 1, maximum = 2048)]
    pub max_tokens: Option<u32>,
    /// 温度参数，默认值为0.7，范围0.0-2.0
    #[schema(example = 0.7, minimum = 0.0, maximum = 2.0)]
    pub temperature: Option<f32>,
}

impl PredictionRequest {
    pub fn validate(&self) -> Result<(), String> {
        // 验证 model 字段
        if self.model.is_empty() {
            return Err("model field is required".to_string());
        }
        
        if self.model != "deepseek" && self.model != "qwen" {
            return Err("model must be either 'deepseek' or 'qwen'".to_string());
        }
        
        // 验证 prompt 字段
        if self.prompt.is_empty() {
            return Err("prompt field is required".to_string());
        }
        
        if self.prompt.len() > 10000 {
            return Err("prompt length must be less than or equal to 10000 characters".to_string());
        }
        
        // 验证 max_tokens 字段
        if let Some(max_tokens) = self.max_tokens {
            if max_tokens == 0 {
                return Err("max_tokens must be greater than 0".to_string());
            }
            if max_tokens > 2048 {
                return Err("max_tokens must be less than or equal to 2048".to_string());
            }
        }
        
        // 验证 temperature 字段
        if let Some(temperature) = self.temperature {
            if temperature < 0.0 {
                return Err("temperature must be greater than or equal to 0.0".to_string());
            }
            if temperature > 2.0 {
                return Err("temperature must be less than or equal to 2.0".to_string());
            }
        }
        
        Ok(())
    }
}

/// 预测响应模型
#[derive(Debug, Deserialize, Serialize, Clone, ToSchema, PartialEq)]
pub struct PredictionResponse {
    /// 使用的模型名称
    #[schema(example = "deepseek")]
    pub model: String,
    /// 生成的文本
    #[schema(example = "Once upon a time, there was an AI that...")]
    pub generated_text: String,
    /// 使用的token数
    #[schema(example = 50)]
    pub tokens_used: u32,
}

#[cfg(test)]
mod tests {
    use super::PredictionRequest;

    #[test]
    fn test_prediction_request_validate_valid() {
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        assert_eq!(request.validate(), Ok(()));
    }

    #[test]
    fn test_prediction_request_validate_invalid_model() {
        let request = PredictionRequest {
            model: "invalid".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_prediction_request_validate_empty_model() {
        let request = PredictionRequest {
            model: "".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_prediction_request_validate_empty_prompt() {
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: "".to_string(),
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_prediction_request_validate_long_prompt() {
        let long_prompt = "a".repeat(10001);
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: long_prompt,
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_prediction_request_validate_invalid_max_tokens() {
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(0),
            temperature: Some(0.7),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_prediction_request_validate_max_tokens_too_large() {
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(2049),
            temperature: Some(0.7),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_prediction_request_validate_invalid_temperature() {
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(100),
            temperature: Some(-0.1),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_prediction_request_validate_temperature_too_high() {
        let request = PredictionRequest {
            model: "deepseek".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: Some(100),
            temperature: Some(2.1),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_prediction_request_validate_with_defaults() {
        let request = PredictionRequest {
            model: "qwen".to_string(),
            prompt: "test prompt".to_string(),
            max_tokens: None,
            temperature: None,
        };
        assert_eq!(request.validate(), Ok(()));
    }
}
