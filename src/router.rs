use crate::{
    billing::BillingManager,
    cache::RedisCache,
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
    pub cache: Option<Arc<RedisCache>>,
}

impl OmenRouter {
    pub async fn new(config: Config) -> Result<Self> {
        let providers = Arc::new(ProviderRegistry::new(&config).await?);
        let advanced_router = Arc::new(tokio::sync::Mutex::new(AdvancedRouter::new()));
        let billing_manager = Arc::new(BillingManager::new());
        let rate_limiter = Arc::new(AdaptiveRateLimiter::new(billing_manager.clone()));

        // Initialize Redis cache if enabled
        let cache = if config.cache.enabled {
            match RedisCache::new(crate::cache::CacheConfig {
                redis_url: config.cache.redis_url.clone(),
                default_ttl_seconds: config.cache.default_ttl_seconds,
                response_cache_ttl: config.cache.response_cache_ttl,
                session_cache_ttl: config.cache.session_cache_ttl,
                rate_limit_ttl: config.cache.rate_limit_ttl,
                provider_health_ttl: config.cache.provider_health_ttl,
                max_cache_size_mb: config.cache.max_cache_size_mb,
            }) {
                Ok(cache) => {
                    cache.warm_cache().await?;
                    info!("🔥 Redis cache initialized and warmed up");
                    Some(Arc::new(cache))
                }
                Err(e) => {
                    warn!("Failed to initialize Redis cache: {}. Continuing without cache.", e);
                    None
                }
            }
        } else {
            info!("📦 Redis cache disabled in configuration");
            None
        };

        info!("✅ OMEN router initialized with {} providers", providers.len());

        Ok(Self {
            config,
            providers,
            advanced_router,
            billing_manager,
            rate_limiter,
            cache,
        })
    }

    pub async fn chat_completion(&self, request: ChatCompletionRequest, context: RequestContext) -> Result<ChatCompletionResponse> {
        // Check cache first for identical requests
        if let (Some(cache), Some(user_id)) = (&self.cache, &context.user_id) {
            let cache_key = cache.generate_response_cache_key(
                user_id,
                &request.messages,
                &request.model,
                request.temperature,
            );

            if let Ok(Some(cached)) = cache.get_cached_response(&cache_key).await {
                info!("💾 Cache HIT for user {}: ${:.4} saved", user_id, cached.cost_usd);

                // Update cache hit count
                let _ = cache.increment_cache_hit(&cache_key).await;

                return Ok(cached.response);
            }
        }

        // Check billing and rate limits
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

        // Record usage for billing and cache the response
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

            // Cache the response for future identical requests
            if let Some(cache) = &self.cache {
                let cache_key = cache.generate_response_cache_key(
                    user_id,
                    &request.messages,
                    &request.model,
                    request.temperature,
                );

                if let Err(e) = cache.cache_response(
                    &cache_key,
                    &response,
                    provider.id(),
                    provider_cost,
                ).await {
                    warn!("Failed to cache response: {}", e);
                }
            }
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

    pub async fn embeddings(&self, request: EmbeddingsRequest, context: RequestContext) -> Result<EmbeddingsResponse> {
        // Check rate limits
        if let Some(ref user_id) = context.user_id {
            let can_proceed = self.billing_manager.check_request_allowed(user_id).await?;
            if !can_proceed {
                return Err(OmenError::RateLimitExceeded);
            }
            // Estimate tokens for embeddings (rough approximation)
            let estimated_tokens = match &request.input {
                EmbeddingInput::Single(text) => (text.len() / 4) as u32,
                EmbeddingInput::Multiple(texts) => texts.iter().map(|t| (t.len() / 4) as u32).sum(),
            };
            let _ = self.rate_limiter.check_rate_limit(user_id, estimated_tokens).await;
        }

        // Select provider (prefer OpenAI/Ollama for embeddings)
        let provider = self.providers.get("openai")
            .or_else(|| self.providers.get("ollama"))
            .or_else(|| self.providers.all().first().cloned())
            .ok_or(OmenError::ProviderUnavailable("No providers available for embeddings".to_string()))?;

        // Convert input to vector of strings
        let texts = match request.input {
            EmbeddingInput::Single(text) => vec![text],
            EmbeddingInput::Multiple(texts) => texts,
        };

        // Generate embeddings (simplified - actual implementation would call provider)
        let mut data = Vec::new();
        for (i, text) in texts.iter().enumerate() {
            // TODO: Call actual provider embedding API
            // For now, return mock embeddings (1536 dimensions for OpenAI compatibility)
            let embedding = vec![0.0; 1536];
            data.push(EmbeddingData {
                object: "embedding".to_string(),
                embedding,
                index: i,
            });
        }

        let total_tokens: u32 = texts.iter().map(|t| (t.len() / 4) as u32).sum();

        Ok(EmbeddingsResponse {
            object: "list".to_string(),
            data,
            model: request.model.clone(),
            usage: EmbeddingUsage {
                prompt_tokens: total_tokens,
                total_tokens,
            },
        })
    }

    pub async fn text_completion(&self, request: CompletionRequest, context: RequestContext) -> Result<CompletionResponse> {
        // Convert completion request to chat completion request
        let prompts = match request.prompt {
            CompletionPrompt::Single(text) => vec![text],
            CompletionPrompt::Multiple(texts) => texts,
        };

        let mut all_responses = Vec::new();

        for prompt in prompts {
            let chat_request = ChatCompletionRequest {
                model: request.model.clone(),
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: MessageContent::Text(prompt),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                }],
                temperature: request.temperature,
                max_tokens: request.max_tokens,
                stream: false,
                top_p: request.top_p,
                frequency_penalty: None,
                presence_penalty: None,
                stop: request.stop.clone(),
                tools: None,
                tool_choice: None,
                tags: None,
                omen: None,
            };

            let chat_response = self.chat_completion(chat_request, context.clone()).await?;

            if let Some(choice) = chat_response.choices.first() {
                all_responses.push(choice.message.content.text().to_string());
            }
        }

        let usage = Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };

        Ok(CompletionResponse {
            id: format!("cmpl-{}", uuid::Uuid::new_v4()),
            object: "text_completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: request.model,
            choices: all_responses.into_iter().enumerate().map(|(i, text)| CompletionChoice {
                text,
                index: i as u32,
                finish_reason: Some("stop".to_string()),
            }).collect(),
            usage,
        })
    }

    pub async fn get_provider_health(&self) -> Result<Vec<ProviderHealth>> {
        let mut health_status = Vec::new();

        for provider in self.providers.all() {
            let start = std::time::Instant::now();
            let healthy = provider.health_check().await.unwrap_or(false);
            let latency_ms = start.elapsed().as_millis() as u64;

            let models = provider.list_models().await.unwrap_or_default();

            // Calculate average cost (simplified)
            let avg_cost = if !models.is_empty() {
                let total: f64 = models.iter().map(|m| m.pricing.input_per_1k).sum();
                Some(total / models.len() as f64)
            } else {
                None
            };

            health_status.push(ProviderHealth {
                id: provider.id().to_string(),
                name: provider.name().to_string(),
                healthy,
                models_count: models.len(),
                latency_ms: Some(latency_ms),
                avg_cost_per_1k: avg_cost,
                success_rate: Some(if healthy { 1.0 } else { 0.0 }),
            });
        }

        Ok(health_status)
    }

    pub async fn get_provider_scores(&self) -> Result<Vec<ProviderScore>> {
        let health_status = self.get_provider_health().await?;
        let mut scores = Vec::new();

        for health in health_status {
            let latency_ms = health.latency_ms.unwrap_or(5000);
            let avg_cost = health.avg_cost_per_1k.unwrap_or(10.0);

            // Scoring algorithm for Zeke
            let health_score = if health.healthy { 100.0 } else { 0.0 };

            // Lower latency = higher score (max 100)
            let latency_score = ((5000.0 - latency_ms as f64) / 50.0).max(0.0).min(100.0);

            // Lower cost = higher score (max 100)
            let cost_score = ((20.0 - avg_cost) / 0.2).max(0.0).min(100.0);

            let reliability_score = health.success_rate.unwrap_or(0.0) * 100.0;

            // Weighted average: health 40%, latency 30%, cost 20%, reliability 10%
            let overall_score = (health_score * 0.4)
                + (latency_score * 0.3)
                + (cost_score * 0.2)
                + (reliability_score * 0.1);

            scores.push(ProviderScore {
                provider_id: health.id.clone(),
                provider_name: health.name.clone(),
                health_score,
                latency_ms,
                cost_score,
                reliability_score,
                overall_score,
                recommended: overall_score > 60.0 && health.healthy,
            });
        }

        // Sort by overall score (descending)
        scores.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap());

        Ok(scores)
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
        // Rough approximation: 4 characters per token for text
        // Images add significant tokens based on resolution
        let mut total_tokens = 0u32;

        for message in &request.messages {
            // Text content
            total_tokens += (message.content.text().len() / 4) as u32;

            // Vision tokens (images are expensive!)
            if let MessageContent::Parts(parts) = &message.content {
                for part in parts {
                    if let ContentPart::ImageUrl { image_url } = part {
                        let image_tokens = match image_url.detail.as_deref() {
                            Some("low") => 85,   // Low detail
                            Some("high") => 765, // High detail
                            _ => 425, // Auto/default
                        };
                        total_tokens += image_tokens;
                    }
                }
            }
        }

        total_tokens
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
                cache: self.cache.clone(),
            }),
            self.billing_manager.clone()
        ))
    }

    pub async fn process_ghost_request(&self, request: crate::ghost_ai::GhostRequest) -> Result<crate::ghost_ai::GhostResponse> {
        let ghost_orchestrator = self.create_ghost_orchestrator();
        ghost_orchestrator.process_ghost_request(request).await
    }
}