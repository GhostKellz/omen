use thiserror::Error;

#[derive(Error, Debug)]
pub enum OmenError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, OmenError>;

// Convert to HTTP response
impl axum::response::IntoResponse for OmenError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;

        let (status, error_message) = match self {
            OmenError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            OmenError::ModelNotFound(msg) => (StatusCode::NOT_FOUND, msg),
            OmenError::ProviderUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            OmenError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string()),
            OmenError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "type": "api_error",
                "code": status.as_u16()
            }
        }));

        (status, body).into_response()
    }
}