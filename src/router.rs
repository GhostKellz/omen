use crate::{
    billing::BillingManager,
    config::Config,
    error::{OmenError, Result},
    ghost_ai::GhostOrchestrator,
    multiplexer::{MultiplexStrategy, StreamMultiplexer},
    providers::{Provider, ProviderRegistry},
    rate_limiter::AdaptiveRateLimiter,
    routing::AdvancedRouter,
    types::*,
};
use futures::stream::Stream;
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug)]
pub struct OmenRouter {
    config: Config,
    providers: Arc<ProviderRegistry>,
    advanced_router: Arc<tokio::sync::Mutex<AdvancedRouter>>,
    billing_manager: Arc<BillingManager>,
    rate_limiter: Arc<AdaptiveRateLimiter>,
}

impl OmenRouter {
    pub async fn new(config: Config) -> Result<Self> {
        let providers = Arc::new(ProviderRegistry::new(&config).await?);
        let advanced_router = Arc::new(tokio::sync::Mutex::new(AdvancedRouter::new()));
        let billing_manager = Arc::new(BillingManager::new());
        let rate_limiter = Arc::new(AdaptiveRateLimiter::new(billing_manager.clone()));

        info!("✅ OMEN router initialized with {} providers", providers.len());

        Ok(Self {
            config,
            providers,
            advanced_router,
            billing_manager,
            rate_limiter,
        })
    }

    pub async fn chat_completion(&self, request: ChatCompletionRequest, context: RequestContext) -> Result<ChatCompletionResponse> {
        // Check billing and rate limits first
        if let Some(ref user_id) = context.user_id {
            let can_proceed = self.billing_manager.check_request_allowed(user_id).await?;
            if !can_proceed {
                return Err(OmenError::RateLimitExceeded);
            }

            // Check rate limits
            let estimated_tokens = self.estimate_input_tokens(&request);
            self.rate_limiter.check_rate_limit(user_id, estimated_tokens).await?;
        }

        let provider = self.select_provider(&request.model, &context).await?;

        info!(
            "🎯 Routing request {} to provider {} for model {}",
            context.request_id, provider.name(), request.model
        );

        let start_time = std::time::Instant::now();
        let response = provider.chat_completion(&request, &context).await?;
        let latency_ms = start_time.elapsed().as_millis() as u64;

        // Record usage for billing
        if let Some(ref user_id) = context.user_id {
            let input_tokens = self.estimate_input_tokens(&request);
            let output_tokens = response.usage.completion_tokens;

            // Estimate cost (would be more accurate with actual provider pricing)
            let provider_cost = self.estimate_provider_cost(provider.id(), input_tokens + output_tokens);

            self.billing_manager.record_usage(
                user_id,
                input_tokens,
                output_tokens,
                provider_cost,
            ).await?;

            // Update router metrics
            self.update_provider_metrics(
                provider.id(),
                latency_ms,
                true,
                provider_cost,
                input_tokens + output_tokens,
            ).await;
        }

        Ok(response)
    }

    pub async fn stream_chat_completion(
        &self,
        request: ChatCompletionRequest,
        context: RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        // Check if OMEN config exists and determine strategy
        if let Some(ref omen_config) = request.omen {
            let strategy = MultiplexStrategy::from(omen_config);
            let candidates = self.select_candidates(&request, &context, omen_config).await?;

            info!(
                "🚀 Multiplexing request {} with strategy {:?} across {} providers",
                context.request_id,
                strategy,
                candidates.len()
            );

            let multiplexer = StreamMultiplexer::new(candidates, omen_config);
            multiplexer.multiplex_stream(request, context, strategy).await
        } else {
            // Fallback to single provider
            let provider = self.select_provider(&request.model, &context).await?;

            info!(
                "🌊 Streaming request {} to provider {} for model {}",
                context.request_id, provider.name(), request.model
            );

            provider.stream_chat_completion(&request, &context).await
        }
    }

    pub async fn list_models(&self) -> Result<Vec<Model>> {
        let mut all_models = Vec::new();

        for provider in self.providers.all() {
            match provider.list_models().await {
                Ok(models) => all_models.extend(models),
                Err(e) => {
                    warn!("Failed to get models from {}: {}", provider.name(), e);
                }
            }
        }

        Ok(all_models)
    }

    pub async fn get_provider_health(&self) -> Result<Vec<ProviderHealth>> {
        let mut health_status = Vec::new();

        for provider in self.providers.all() {
            let healthy = provider.health_check().await.unwrap_or(false);
            let models = provider.list_models().await.unwrap_or_default();

            health_status.push(ProviderHealth {
                id: provider.id().to_string(),
                name: provider.name().to_string(),
                healthy,
                models_count: models.len(),
            });
        }

        Ok(health_status)
    }

    pub async fn check_provider_health(&self, provider_id: &str) -> Result<bool> {
        if let Some(provider) = self.providers.get(provider_id) {
            Ok(provider.health_check().await.unwrap_or(false))
        } else {
            Err(OmenError::ProviderUnavailable(format!(
                "Provider {} not found",
                provider_id
            )))
        }
    }

    pub async fn list_providers(&self) -> Vec<Arc<dyn Provider>> {
        self.providers.all()
    }

    fn create_request_context(&self, request: &ChatCompletionRequest) -> RequestContext {
        let mut tags = HashMap::new();

        // Extract intent from request
        let intent = self.classify_intent(request);
        if let Some(ref intent_str) = intent {
            tags.insert("intent".to_string(), intent_str.clone());
        }

        // Add user-provided tags
        if let Some(ref user_tags) = request.tags {
            tags.extend(user_tags.clone());
        }

        RequestContext {
            request_id: Uuid::new_v4(),
            user_id: None, // TODO: Extract from auth headers
            api_key: None, // TODO: Extract from auth headers
            intent,
            tags,
        }
    }

    fn classify_intent(&self, request: &ChatCompletionRequest) -> Option<String> {
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

    async fn select_provider(&self, model: &str, context: &RequestContext) -> Result<Arc<dyn Provider>> {
        // Handle special model names
        if model == "auto" {
            return self.select_auto_provider(context).await;
        }

        // Try to find provider by exact model match
        for provider in self.providers.all() {
            if let Ok(models) = provider.list_models().await {
                if models.iter().any(|m| m.id == model) {
                    if provider.health_check().await.unwrap_or(false) {
                        return Ok(provider);
                    }
                }
            }
        }

        Err(OmenError::ModelNotFound(format!("Model {} not found or provider unavailable", model)))
    }

    async fn select_auto_provider(&self, context: &RequestContext) -> Result<Arc<dyn Provider>> {
        let intent = context.intent.as_deref().unwrap_or("general");

        // Check if we should prefer local models for this intent
        if self.config.routing.prefer_local_for.contains(&intent.to_string()) {
            // Try Ollama first for local models
            if let Some(ollama) = self.providers.get("ollama") {
                if ollama.health_check().await.unwrap_or(false) {
                    return Ok(ollama);
                }
            }
        }

        // Fallback to cloud providers based on availability and routing rules
        let cloud_providers = ["openai", "anthropic", "google", "azure", "xai"];

        for provider_id in &cloud_providers {
            if let Some(provider) = self.providers.get(provider_id) {
                if provider.health_check().await.unwrap_or(false) {
                    return Ok(provider);
                }
            }
        }

        Err(OmenError::ProviderUnavailable("No providers available".to_string()))
    }

    async fn select_candidates(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
        omen_config: &OmenConfig,
    ) -> Result<Vec<Arc<dyn Provider>>> {
        let mut candidates = Vec::new();

        // If specific providers are requested, use those
        if let Some(ref provider_list) = omen_config.providers {
            for provider_id in provider_list {
                if let Some(provider) = self.providers.get(provider_id) {
                    if provider.health_check().await.unwrap_or(false) {
                        candidates.push(provider);
                    }
                }
            }
        } else {
            // Use smart selection based on intent and model
            if request.model == "auto" {
                // For auto model, use intent-based selection
                candidates = self.select_candidates_by_intent(context).await?;
            } else {
                // For specific model, find providers that support it
                candidates = self.select_candidates_by_model(&request.model).await?;
            }
        }

        if candidates.is_empty() {
            return Err(OmenError::ProviderUnavailable("No suitable providers found".to_string()));
        }

        // Use advanced routing for optimal selection
        let k = omen_config.k.unwrap_or(2) as usize;
        let advanced_router = self.advanced_router.lock().await;

        match advanced_router.select_optimal_providers(&candidates, request, context, k).await {
            Ok(decision) => {
                info!("🧠 Advanced routing decision: {}", decision.strategy_used);
                info!("💰 Estimated cost: ${:.4}, latency: {}ms, confidence: {:.2}",
                      decision.estimated_cost, decision.estimated_latency_ms, decision.confidence_score);

                // Return providers in optimal order
                let mut optimal_candidates = Vec::new();
                for provider_id in &decision.selected_providers {
                    if let Some(provider) = self.providers.get(provider_id) {
                        optimal_candidates.push(provider);
                    }
                }
                Ok(optimal_candidates)
            }
            Err(_) => {
                // Fallback to simple priority weights if advanced routing fails
                if let Some(ref weights) = omen_config.priority_weights {
                    candidates.sort_by(|a, b| {
                        let weight_a = weights.get(a.id()).unwrap_or(&1.0);
                        let weight_b = weights.get(b.id()).unwrap_or(&1.0);
                        weight_b.partial_cmp(weight_a).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                Ok(candidates.into_iter().take(k).collect())
            }
        }
    }

    async fn select_candidates_by_intent(&self, context: &RequestContext) -> Result<Vec<Arc<dyn Provider>>> {
        let intent = context.intent.as_deref().unwrap_or("general");
        let mut candidates = Vec::new();

        // Start with local if preferred for this intent
        if self.config.routing.prefer_local_for.contains(&intent.to_string()) {
            if let Some(ollama) = self.providers.get("ollama") {
                if ollama.health_check().await.unwrap_or(false) {
                    candidates.push(ollama);
                }
            }
        }

        // Add cloud providers
        let cloud_providers = ["anthropic", "openai", "google", "azure", "xai"];
        for provider_id in &cloud_providers {
            if let Some(provider) = self.providers.get(provider_id) {
                if provider.health_check().await.unwrap_or(false) {
                    candidates.push(provider);
                }
            }
        }

        Ok(candidates)
    }

    async fn select_candidates_by_model(&self, model: &str) -> Result<Vec<Arc<dyn Provider>>> {
        let mut candidates = Vec::new();

        // Find all providers that support this model
        for provider in self.providers.all() {
            if let Ok(models) = provider.list_models().await {
                if models.iter().any(|m| m.id == model) {
                    if provider.health_check().await.unwrap_or(false) {
                        candidates.push(provider);
                    }
                }
            }
        }

        Ok(candidates)
    }

    pub async fn update_provider_metrics(
        &self,
        provider_id: &str,
        latency_ms: u64,
        success: bool,
        cost_usd: f64,
        tokens_used: u32,
    ) {
        let mut advanced_router = self.advanced_router.lock().await;
        advanced_router.update_metrics_from_response(provider_id, latency_ms, success, cost_usd, tokens_used);
    }

    pub async fn set_user_budget(&self, user_id: &str, budget_usd: f64) {
        let mut advanced_router = self.advanced_router.lock().await;
        advanced_router.set_user_budget(user_id, budget_usd);
    }

    fn estimate_input_tokens(&self, request: &ChatCompletionRequest) -> u32 {
        // Rough approximation: 4 characters per token
        let total_chars: usize = request.messages.iter()
            .map(|m| m.content.len())
            .sum();
        (total_chars / 4) as u32
    }

    fn estimate_provider_cost(&self, provider_id: &str, total_tokens: u32) -> f64 {
        // Use known provider pricing (should match routing.rs defaults)
        let cost_per_1k = match provider_id {
            "ollama" => 0.0,        // Free local
            "openai" => 0.03,       // GPT-4 pricing
            "anthropic" => 0.015,   // Claude pricing
            "google" => 0.00125,    // Gemini pricing
            "azure" => 0.03,        // Azure OpenAI
            "xai" => 0.0,           // Free during beta
            "bedrock" => 0.015,     // AWS Bedrock
            _ => 0.02,              // Default fallback
        };

        (total_tokens as f64 / 1000.0) * cost_per_1k
    }

    // Billing management methods
    pub async fn get_user_usage_stats(&self, user_id: &str) -> Result<crate::billing::UserUsageStats> {
        self.billing_manager.get_user_usage_stats(user_id).await
    }

    pub async fn update_user_tier(&self, user_id: &str, tier_name: &str) -> Result<()> {
        self.billing_manager.update_user_tier(user_id, tier_name).await
    }

    pub async fn get_billing_summary(&self) -> Vec<crate::billing::UserBillingSummary> {
        self.billing_manager.get_all_users_summary().await
    }

    pub fn get_available_billing_tiers(&self) -> Vec<&crate::billing::BillingTier> {
        self.billing_manager.list_available_tiers()
    }

    // Rate limiting methods
    pub async fn get_rate_limit_status(&self, user_id: &str) -> crate::rate_limiter::RateLimitStatus {
        self.rate_limiter.get_rate_limit_status(user_id).await
    }

    pub async fn cleanup_rate_limit_buckets(&self) {
        self.rate_limiter.cleanup_expired_buckets().await
    }

    // Ghost AI integration methods
    pub fn create_ghost_orchestrator(&self) -> Arc<GhostOrchestrator> {
        Arc::new(GhostOrchestrator::new(
            // Create a weak reference to avoid circular dependency
            Arc::new(Self {
                config: self.config.clone(),
                providers: self.providers.clone(),
                advanced_router: self.advanced_router.clone(),
                billing_manager: self.billing_manager.clone(),
                rate_limiter: self.rate_limiter.clone(),
            }),
            self.billing_manager.clone()
        ))
    }

    pub async fn process_ghost_request(&self, request: crate::ghost_ai::GhostRequest) -> Result<crate::ghost_ai::GhostResponse> {
        let ghost_orchestrator = self.create_ghost_orchestrator();
        ghost_orchestrator.process_ghost_request(request).await
    }
}