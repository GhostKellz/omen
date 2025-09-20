use crate::{
    config::Config,
    error::{OmenError, Result},
    types::RequestContext,
};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, warn};

pub struct AuthService {
    config: Arc<Config>,
    api_keys: HashMap<String, ApiKeyInfo>,
}

#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub user_id: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub rate_limit_per_hour: Option<u32>,
    pub budget_usd_per_day: Option<f64>,
    pub allowed_models: Option<Vec<String>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

impl AuthService {
    pub fn new(config: Arc<Config>) -> Self {
        let mut api_keys = HashMap::new();

        // Add master key if configured
        if let Some(ref master_key) = config.auth.master_key {
            api_keys.insert(
                master_key.clone(),
                ApiKeyInfo {
                    user_id: "admin".to_string(),
                    name: "Master Key".to_string(),
                    permissions: vec!["*".to_string()],
                    rate_limit_per_hour: None,
                    budget_usd_per_day: None,
                    allowed_models: None,
                    created_at: chrono::Utc::now(),
                    last_used: None,
                },
            );
        }

        // TODO: Load additional API keys from database/config

        Self { config, api_keys }
    }

    pub fn extract_auth_info(&self, headers: &HeaderMap) -> Option<ApiKeyInfo> {
        // Try different authentication methods
        if let Some(api_key) = self.extract_bearer_token(headers) {
            return self.api_keys.get(&api_key).cloned();
        }

        if let Some(api_key) = self.extract_api_key_header(headers) {
            return self.api_keys.get(&api_key).cloned();
        }

        None
    }

    fn extract_bearer_token(&self, headers: &HeaderMap) -> Option<String> {
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|auth| {
                if auth.starts_with("Bearer ") {
                    Some(auth[7..].to_string())
                } else {
                    None
                }
            })
    }

    fn extract_api_key_header(&self, headers: &HeaderMap) -> Option<String> {
        headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    pub fn validate_permissions(&self, auth_info: &ApiKeyInfo, action: &str) -> bool {
        // Master permission grants everything
        if auth_info.permissions.contains(&"*".to_string()) {
            return true;
        }

        // Check specific permissions
        auth_info.permissions.contains(&action.to_string())
    }

    pub fn validate_model_access(&self, auth_info: &ApiKeyInfo, model: &str) -> bool {
        // If no model restrictions, allow all
        if auth_info.allowed_models.is_none() {
            return true;
        }

        // Check if model is in allowed list
        auth_info
            .allowed_models
            .as_ref()
            .map(|models| models.contains(&model.to_string()))
            .unwrap_or(true)
    }
}

// Middleware function for authentication
pub async fn auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut request: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // Skip auth if not required
    if !auth_service.config.auth.require_api_key {
        return Ok(next.run(request).await);
    }

    let headers = request.headers();

    // Extract and validate authentication
    if let Some(auth_info) = auth_service.extract_auth_info(headers) {
        debug!("Authenticated request from user: {}", auth_info.user_id);

        // Add auth info to request extensions for later use
        request.extensions_mut().insert(auth_info);

        Ok(next.run(request).await)
    } else {
        warn!("Unauthorized request - missing or invalid API key");
        Err(StatusCode::UNAUTHORIZED)
    }
}

// Helper to extract auth info from request extensions
pub fn get_auth_info(request: &Request) -> Option<&ApiKeyInfo> {
    request.extensions().get::<ApiKeyInfo>()
}

// Helper to create authenticated request context
pub fn create_authenticated_context(
    auth_info: Option<&ApiKeyInfo>,
    request: &crate::types::ChatCompletionRequest,
) -> RequestContext {
    use uuid::Uuid;
    use std::collections::HashMap;

    let mut tags = HashMap::new();

    // Add authentication-related tags
    if let Some(auth) = auth_info {
        tags.insert("user_id".to_string(), auth.user_id.clone());
        tags.insert("api_key_name".to_string(), auth.name.clone());
    }

    // Add request-specific tags from OMEN extensions
    if let Some(ref omen_config) = request.omen {
        if let Some(ref strategy) = omen_config.strategy {
            tags.insert("strategy".to_string(), strategy.clone());
        }
        if let Some(k) = omen_config.k {
            tags.insert("k".to_string(), k.to_string());
        }
        if let Some(budget) = omen_config.budget_usd {
            tags.insert("budget_usd".to_string(), budget.to_string());
        }
    }

    RequestContext {
        request_id: Uuid::new_v4(),
        user_id: auth_info.map(|a| a.user_id.clone()),
        api_key: None, // Don't store the actual key
        intent: classify_request_intent(request),
        tags,
    }
}

fn classify_request_intent(request: &crate::types::ChatCompletionRequest) -> Option<String> {
    // Simple intent classification based on message content
    if let Some(last_message) = request.messages.last() {
        let content = last_message.content.to_lowercase();

        if content.contains("code") || content.contains("function") || content.contains("implement") {
            return Some("code".to_string());
        }
        if content.contains("test") || content.contains("unit test") {
            return Some("tests".to_string());
        }
        if content.contains("regex") || content.contains("pattern") {
            return Some("regex".to_string());
        }
        if content.contains("analyze") || content.contains("review") {
            return Some("analysis".to_string());
        }
        if content.contains("explain") || content.contains("summarize") {
            return Some("explanation".to_string());
        }
    }

    Some("general".to_string())
}

// Rate limiting structures
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_hour: u32,
    pub tokens_per_hour: u32,
    pub requests_per_day: u32,
    pub tokens_per_day: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageTracker {
    pub requests_today: u32,
    pub tokens_today: u32,
    pub cost_today_usd: f64,
    pub last_reset: chrono::DateTime<chrono::Utc>,
}

impl Default for UsageTracker {
    fn default() -> Self {
        Self {
            requests_today: 0,
            tokens_today: 0,
            cost_today_usd: 0.0,
            last_reset: chrono::Utc::now(),
        }
    }
}

impl UsageTracker {
    pub fn should_reset_daily(&self) -> bool {
        let now = chrono::Utc::now();
        now.date_naive() > self.last_reset.date_naive()
    }

    pub fn reset_daily(&mut self) {
        self.requests_today = 0;
        self.tokens_today = 0;
        self.cost_today_usd = 0.0;
        self.last_reset = chrono::Utc::now();
    }

    pub fn add_usage(&mut self, tokens: u32, cost_usd: f64) {
        if self.should_reset_daily() {
            self.reset_daily();
        }

        self.requests_today += 1;
        self.tokens_today += tokens;
        self.cost_today_usd += cost_usd;
    }

    pub fn check_budget_limit(&self, budget_limit: f64) -> bool {
        self.cost_today_usd < budget_limit
    }
}