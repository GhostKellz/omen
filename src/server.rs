use crate::{auth, config::Config, error::Result, router::OmenRouter, types::*};
use axum::{
    extract::{Extension, Path, Query, State},
    http::Method,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::{collections::HashMap, sync::Arc};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};
use uuid::Uuid;

pub struct Server {
    config: Config,
    router: Arc<OmenRouter>,
    auth_service: Arc<auth::AuthService>,
}

impl Server {
    pub async fn new(config: Config) -> Result<Self> {
        let router = Arc::new(OmenRouter::new(config.clone()).await?);
        let auth_service = Arc::new(auth::AuthService::new(Arc::new(config.clone())));

        Ok(Self { config, router, auth_service })
    }

    pub async fn start(self) -> Result<()> {
        let http_app = self.create_app();

        let http_addr = format!("{}:{}", self.config.server.bind, self.config.server.port);

        info!("🌟 OMEN HTTP API listening on {}", http_addr);
        // TODO: Re-enable gRPC server after fixing compilation issues
        // info!("🚀 OMEN gRPC API will be available on port {}", self.config.server.port + 1);

        let listener = tokio::net::TcpListener::bind(&http_addr).await?;
        axum::serve(listener, http_app).await?;

        Ok(())
    }

    fn create_app(&self) -> Router {
        // Public routes (no auth required)
        let public_routes = Router::new()
            .route("/health", get(health_check))
            .route("/status", get(status_check))
            .with_state(Arc::clone(&self.router));

        // Protected routes (auth required)
        let protected_routes = Router::new()
            .route("/v1/models", get(list_models))
            .route("/v1/chat/completions", post(chat_completions))
            .route("/v1/completions", post(completions))
            .route("/omen/providers", get(list_providers))
            .route("/omen/providers/:id/health", get(provider_health))
            .route("/admin/usage", get(usage_stats))
            .route("/admin/config", get(config_info))
            .route("/billing/usage", get(user_usage_stats))
            .route("/billing/tiers", get(billing_tiers))
            .route("/billing/tier", post(update_user_tier))
            .route("/admin/billing/summary", get(billing_summary))
            .route("/rate-limit/status", get(rate_limit_status))
            .route("/ghost/chat/completions", post(ghost_chat_completions))
            .route("/ghost/session/:session_id/stats", get(ghost_session_stats))
            .route("/cache/stats", get(cache_stats))
            .route("/cache/clear", post(cache_clear))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&self.auth_service),
                auth::auth_middleware,
            ))
            .with_state(Arc::clone(&self.router));

        // Combine routes and add global middleware
        Router::new()
            .merge(public_routes)
            .merge(protected_routes)
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                    .allow_headers(Any),
            )
    }
}

// Route handlers
async fn health_check(State(router): State<Arc<OmenRouter>>) -> Result<Json<HealthResponse>> {
    let providers = router.get_provider_health().await?;

    let response = HealthResponse {
        status: if providers.iter().any(|p| p.healthy) {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "OMEN".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        providers,
    };

    Ok(Json(response))
}

async fn status_check(State(router): State<Arc<OmenRouter>>) -> Result<Json<serde_json::Value>> {
    let providers = router.get_provider_health().await?;
    let models = router.list_models().await?;

    Ok(Json(serde_json::json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": 0, // TODO: track actual uptime
        "providers_count": providers.len(),
        "models_count": models.len(),
        "requests_today": 0, // TODO: track actual requests
        "cache_hit_rate": 0.0, // TODO: track cache hits
    })))
}

async fn list_models(State(router): State<Arc<OmenRouter>>) -> Result<Json<ModelsResponse>> {
    let models = router.list_models().await?;

    Ok(Json(ModelsResponse {
        object: "list".to_string(),
        data: models,
    }))
}

async fn chat_completions(
    State(router): State<Arc<OmenRouter>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Response> {
    // Extract JSON from request body manually
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await
        .map_err(|e| crate::error::OmenError::InvalidRequest(format!("Failed to read request body: {}", e)))?;

    let chat_request: ChatCompletionRequest = serde_json::from_slice(&bytes)
        .map_err(|e| crate::error::OmenError::InvalidRequest(format!("Invalid JSON: {}", e)))?;

    // Extract auth info from request extensions (set by middleware)
    let auth_info = parts.extensions.get::<auth::ApiKeyInfo>();
    let context = auth::create_authenticated_context(auth_info, &chat_request);

    if chat_request.stream {
        // Return SSE stream
        let stream = router.stream_chat_completion(chat_request, context).await?;

        // Convert string stream to text/event-stream format
        let text_stream = futures::stream::unfold(stream, |mut stream| async {
            use futures::StreamExt;
            match stream.next().await {
                Some(Ok(data)) => Some((Ok::<String, std::io::Error>(data), stream)),
                Some(Err(_)) => None, // End stream on error
                None => None,
            }
        });

        let response = Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(axum::body::Body::from_stream(text_stream))
            .unwrap();

        Ok(response)
    } else {
        // Return regular JSON response
        let response = router.chat_completion(chat_request, context).await?;
        Ok(Json(response).into_response())
    }
}

async fn completions(
    State(_router): State<Arc<OmenRouter>>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    // TODO: Implement text completions
    warn!("Text completions not yet implemented");
    Ok(Json(serde_json::json!({
        "error": "Text completions not yet implemented"
    })))
}

async fn list_providers(State(router): State<Arc<OmenRouter>>) -> Result<Json<serde_json::Value>> {
    let providers = router.get_provider_health().await?;

    Ok(Json(serde_json::json!({
        "providers": providers
    })))
}

async fn provider_health(
    State(router): State<Arc<OmenRouter>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let health = router.check_provider_health(&id).await?;

    Ok(Json(serde_json::json!({
        "provider_id": id,
        "healthy": health,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn usage_stats(
    State(_router): State<Arc<OmenRouter>>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    // TODO: Implement usage statistics
    warn!("Usage statistics not yet implemented");

    Ok(Json(serde_json::json!({
        "period": {
            "start": chrono::Utc::now().to_rfc3339(),
            "end": chrono::Utc::now().to_rfc3339()
        },
        "summary": {
            "requests": 0,
            "tokens": 0,
            "cost": 0.0,
            "unique_users": 0
        }
    })))
}

async fn config_info(State(router): State<Arc<OmenRouter>>) -> Result<Json<serde_json::Value>> {
    let providers = router.get_provider_health().await?;

    Ok(Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "providers_enabled": providers.iter().filter(|p| p.healthy).count(),
        "total_providers": providers.len(),
        "features": {
            "streaming": true,
            "function_calling": true,
            "vision": true,
            "embeddings": false
        }
    })))
}

// Billing endpoints
async fn user_usage_stats(
    State(router): State<Arc<OmenRouter>>,
    Extension(auth_info): Extension<crate::auth::ApiKeyInfo>,
) -> Result<Json<crate::billing::UserUsageStats>> {
    let user_id = &auth_info.user_id;
    let stats = router.get_user_usage_stats(user_id).await?;
    Ok(Json(stats))
}

async fn billing_tiers(
    State(router): State<Arc<OmenRouter>>,
) -> Result<Json<Vec<crate::billing::BillingTier>>> {
    let tiers = router.get_available_billing_tiers()
        .into_iter()
        .cloned()
        .collect();
    Ok(Json(tiers))
}

#[derive(serde::Deserialize)]
struct UpdateTierRequest {
    tier: String,
}

async fn update_user_tier(
    State(router): State<Arc<OmenRouter>>,
    Extension(auth_info): Extension<crate::auth::ApiKeyInfo>,
    Json(request): Json<UpdateTierRequest>,
) -> Result<Json<serde_json::Value>> {
    let user_id = &auth_info.user_id;
    router.update_user_tier(user_id, &request.tier).await?;

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "new_tier": request.tier,
        "updated_at": chrono::Utc::now().to_rfc3339()
    })))
}

async fn billing_summary(
    State(router): State<Arc<OmenRouter>>,
) -> Result<Json<Vec<crate::billing::UserBillingSummary>>> {
    let summary = router.get_billing_summary().await;
    Ok(Json(summary))
}

// Rate limiting endpoints
async fn rate_limit_status(
    State(router): State<Arc<OmenRouter>>,
    Extension(auth_info): Extension<crate::auth::ApiKeyInfo>,
) -> Result<Json<crate::rate_limiter::RateLimitStatus>> {
    let status = router.get_rate_limit_status(&auth_info.user_id).await;
    Ok(Json(status))
}

// Ghost AI endpoints
async fn ghost_chat_completions(
    State(router): State<Arc<OmenRouter>>,
    Extension(auth_info): Extension<crate::auth::ApiKeyInfo>,
    Json(request): Json<crate::ghost_ai::GhostRequest>,
) -> Result<Json<crate::ghost_ai::GhostResponse>> {
    let response = router.process_ghost_request(request).await?;
    Ok(Json(response))
}

async fn ghost_session_stats(
    State(router): State<Arc<OmenRouter>>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    // Parse session ID
    let session_uuid = session_id.parse::<uuid::Uuid>()
        .map_err(|_| crate::error::OmenError::InvalidRequest("Invalid session ID".to_string()))?;

    let ghost_orchestrator = router.create_ghost_orchestrator();

    match ghost_orchestrator.get_ghost_session_stats(session_uuid).await {
        Some(stats) => Ok(Json(serde_json::to_value(stats).unwrap())),
        None => Ok(Json(serde_json::json!({
            "error": "Session not found",
            "session_id": session_id
        })))
    }
}

// Cache management endpoints
async fn cache_stats(
    State(router): State<Arc<OmenRouter>>,
) -> Result<Json<serde_json::Value>> {
    match &router.cache {
        Some(cache) => {
            match cache.get_cache_stats().await {
                Ok(stats) => Ok(Json(serde_json::to_value(stats).unwrap())),
                Err(e) => Ok(Json(serde_json::json!({
                    "error": format!("Failed to get cache stats: {}", e),
                    "cache_enabled": true
                })))
            }
        }
        None => Ok(Json(serde_json::json!({
            "cache_enabled": false,
            "message": "Redis cache is not enabled"
        })))
    }
}

#[derive(serde::Deserialize)]
struct CacheClearRequest {
    pattern: Option<String>,
}

async fn cache_clear(
    State(router): State<Arc<OmenRouter>>,
    Json(request): Json<CacheClearRequest>,
) -> Result<Json<serde_json::Value>> {
    match &router.cache {
        Some(cache) => {
            match cache.clear_cache(request.pattern.as_deref()).await {
                Ok(deleted) => Ok(Json(serde_json::json!({
                    "deleted_keys": deleted,
                    "pattern": request.pattern.unwrap_or("*".to_string()),
                    "message": format!("Cleared {} cache entries", deleted)
                }))),
                Err(e) => Ok(Json(serde_json::json!({
                    "error": format!("Failed to clear cache: {}", e),
                    "deleted_keys": 0
                })))
            }
        }
        None => Ok(Json(serde_json::json!({
            "cache_enabled": false,
            "message": "Redis cache is not enabled"
        })))
    }
}