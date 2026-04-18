use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;

// 错误类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum AppErrorType {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    InternalServerError,
    ServiceUnavailable,
}

// 统一的错误响应格式
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub error_type: String,
    pub status_code: u16,
}

// 应用错误结构体
#[derive(Debug)]
pub struct AppError {
    pub error_type: AppErrorType,
    pub message: String,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.error_type, self.message)
    }
}

impl std::error::Error for AppError {}

impl ResponseError for AppError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self.error_type {
            AppErrorType::BadRequest => actix_web::http::StatusCode::BAD_REQUEST,
            AppErrorType::Unauthorized => actix_web::http::StatusCode::UNAUTHORIZED,
            AppErrorType::Forbidden => actix_web::http::StatusCode::FORBIDDEN,
            AppErrorType::NotFound => actix_web::http::StatusCode::NOT_FOUND,
            AppErrorType::InternalServerError => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            AppErrorType::ServiceUnavailable => actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            error: self.message.clone(),
            error_type: format!("{:?}", self.error_type),
            status_code: status_code.as_u16(),
        };

        HttpResponse::build(status_code).json(error_response)
    }
}

// 辅助方法，用于创建不同类型的错误
impl AppError {
    pub fn bad_request(message: &str) -> Self {
        AppError {
            error_type: AppErrorType::BadRequest,
            message: message.to_string(),
        }
    }

    pub fn unauthorized(message: &str) -> Self {
        AppError {
            error_type: AppErrorType::Unauthorized,
            message: message.to_string(),
        }
    }

    pub fn forbidden(message: &str) -> Self {
        AppError {
            error_type: AppErrorType::Forbidden,
            message: message.to_string(),
        }
    }

    pub fn not_found(message: &str) -> Self {
        AppError {
            error_type: AppErrorType::NotFound,
            message: message.to_string(),
        }
    }

    pub fn internal_server_error(message: &str) -> Self {
        AppError {
            error_type: AppErrorType::InternalServerError,
            message: message.to_string(),
        }
    }

    pub fn service_unavailable(message: &str) -> Self {
        AppError {
            error_type: AppErrorType::ServiceUnavailable,
            message: message.to_string(),
        }
    }
}
