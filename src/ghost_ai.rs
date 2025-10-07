use crate::{
    billing::BillingManager,
    error::{OmenError, Result},
    rate_limiter::GhostAIRateLimiter,
    router::OmenRouter,
    types::*,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Ghost AI ecosystem service definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GhostService {
    GhostLLM,     // Main AI interface with OpenWebUI
    GhostFlow,    // Automation and workflow engine
    Zeke,         // Core AI assistant
    Jarvis,       // Orchestration and coordination
    External,     // External API users
    Development,  // Development and testing
}

impl GhostService {
    pub fn as_str(&self) -> &'static str {
        match self {
            GhostService::GhostLLM => "ghostllm",
            GhostService::GhostFlow => "ghostflow",
            GhostService::Zeke => "zeke",
            GhostService::Jarvis => "jarvis",
            GhostService::External => "external",
            GhostService::Development => "development",
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            GhostService::GhostLLM => 255,
            GhostService::GhostFlow => 200,
            GhostService::Zeke => 180,
            GhostService::Jarvis => 160,
            GhostService::External => 100,
            GhostService::Development => 50,
        }
    }
}

/// Ghost AI communication context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostContext {
    pub service: GhostService,
    pub session_id: Uuid,
    pub user_id: String,
    pub workflow_id: Option<Uuid>,
    pub chain_id: Option<Uuid>,    // For chained AI operations
    pub priority: u8,
    pub metadata: HashMap<String, String>,
}

impl GhostContext {
    pub fn new(service: GhostService, user_id: String) -> Self {
        Self {
            priority: service.priority(),
            service,
            session_id: Uuid::new_v4(),
            user_id,
            workflow_id: None,
            chain_id: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_workflow(mut self, workflow_id: Uuid) -> Self {
        self.workflow_id = Some(workflow_id);
        self
    }

    pub fn with_chain(mut self, chain_id: Uuid) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Enhanced request for Ghost AI ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostRequest {
    pub context: GhostContext,
    pub chat_request: ChatCompletionRequest,
    pub routing_hints: Option<GhostRoutingHints>,
    pub fallback_strategy: Option<GhostFallbackStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostRoutingHints {
    pub prefer_local: bool,
    pub max_latency_ms: Option<u64>,
    pub cost_sensitivity: f32,      // 0.0 = cost-insensitive, 1.0 = very cost-sensitive
    pub quality_threshold: f32,     // 0.0-1.0, minimum quality score
    pub provider_whitelist: Option<Vec<String>>,
    pub provider_blacklist: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GhostFallbackStrategy {
    None,
    CheaperProvider,
    LocalOnly,
    CloudOnly,
    BestEffort,
}

/// Response with Ghost AI specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostResponse {
    pub response: ChatCompletionResponse,
    pub ghost_metadata: GhostResponseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostResponseMetadata {
    pub session_id: Uuid,
    pub provider_used: String,
    pub routing_decision: String,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub quality_score: f64,
    pub fallback_used: bool,
    pub rate_limit_remaining: Option<u32>,
}

/// Ghost AI orchestration layer
#[derive(Debug)]
pub struct GhostOrchestrator {
    router: Arc<OmenRouter>,
    rate_limiter: Arc<GhostAIRateLimiter>,
    active_sessions: Arc<RwLock<HashMap<Uuid, GhostSession>>>,
    service_configs: HashMap<String, GhostServiceConfig>,
}

#[derive(Debug, Clone)]
struct GhostSession {
    context: GhostContext,
    created_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
    request_count: u32,
    total_cost: f64,
}

#[derive(Debug, Clone)]
struct GhostServiceConfig {
    default_routing_hints: GhostRoutingHints,
    fallback_strategy: GhostFallbackStrategy,
    session_timeout_minutes: u32,
}

impl GhostOrchestrator {
    pub fn new(router: Arc<OmenRouter>, billing_manager: Arc<BillingManager>) -> Self {
        let rate_limiter = Arc::new(GhostAIRateLimiter::new(billing_manager));

        let mut service_configs = HashMap::new();

        // GhostLLM - prioritize quality and reliability
        service_configs.insert("ghostllm".to_string(), GhostServiceConfig {
            default_routing_hints: GhostRoutingHints {
                prefer_local: false,
                max_latency_ms: Some(3000),
                cost_sensitivity: 0.3,
                quality_threshold: 0.8,
                provider_whitelist: None,
                provider_blacklist: None,
            },
            fallback_strategy: GhostFallbackStrategy::BestEffort,
            session_timeout_minutes: 60,
        });

        // GhostFlow - prioritize speed and cost for automation
        service_configs.insert("ghostflow".to_string(), GhostServiceConfig {
            default_routing_hints: GhostRoutingHints {
                prefer_local: true,
                max_latency_ms: Some(1500),
                cost_sensitivity: 0.7,
                quality_threshold: 0.6,
                provider_whitelist: None,
                provider_blacklist: None,
            },
            fallback_strategy: GhostFallbackStrategy::CheaperProvider,
            session_timeout_minutes: 30,
        });

        // Zeke - balance of quality and speed
        service_configs.insert("zeke".to_string(), GhostServiceConfig {
            default_routing_hints: GhostRoutingHints {
                prefer_local: false,
                max_latency_ms: Some(2000),
                cost_sensitivity: 0.4,
                quality_threshold: 0.75,
                provider_whitelist: None,
                provider_blacklist: None,
            },
            fallback_strategy: GhostFallbackStrategy::BestEffort,
            session_timeout_minutes: 45,
        });

        // Jarvis - orchestration needs reliable responses
        service_configs.insert("jarvis".to_string(), GhostServiceConfig {
            default_routing_hints: GhostRoutingHints {
                prefer_local: false,
                max_latency_ms: Some(2500),
                cost_sensitivity: 0.2,
                quality_threshold: 0.85,
                provider_whitelist: None,
                provider_blacklist: None,
            },
            fallback_strategy: GhostFallbackStrategy::BestEffort,
            session_timeout_minutes: 120,
        });

        Self {
            router,
            rate_limiter,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            service_configs,
        }
    }

    pub async fn process_ghost_request(&self, request: GhostRequest) -> Result<GhostResponse> {
        let start_time = std::time::Instant::now();
        let service_name = request.context.service.as_str();

        info!("🤖 Ghost AI request from {}: session {}",
              service_name, request.context.session_id);

        // Create or update session
        self.update_session(&request.context).await;

        // Check rate limits with Ghost AI priority
        let estimated_tokens = self.estimate_tokens(&request.chat_request);
        self.rate_limiter.check_ghost_service_limit(
            service_name,
            &request.context.user_id,
            estimated_tokens
        ).await?;

        // Apply service-specific routing
        let enhanced_request = self.enhance_request_with_service_config(request).await?;

        // Convert to standard request context
        let request_context = self.create_request_context(&enhanced_request.context);

        // Route and execute request
        let response = self.router.chat_completion(
            enhanced_request.chat_request.clone(),
            request_context
        ).await?;

        let latency_ms = start_time.elapsed().as_millis() as u64;

        // Create Ghost response with metadata
        let ghost_response = GhostResponse {
            response: response.clone(),
            ghost_metadata: GhostResponseMetadata {
                session_id: enhanced_request.context.session_id,
                provider_used: "auto".to_string(), // TODO: Get actual provider from router
                routing_decision: format!("ghost_{}", service_name),
                cost_usd: self.estimate_cost(&response, service_name),
                latency_ms,
                quality_score: 0.85, // TODO: Calculate actual quality score
                fallback_used: false, // TODO: Track if fallback was used
                rate_limit_remaining: None, // TODO: Get from rate limiter
            },
        };

        info!("✅ Ghost AI response for {}: {}ms, ${:.4}",
              service_name, latency_ms, ghost_response.ghost_metadata.cost_usd);

        Ok(ghost_response)
    }

    async fn update_session(&self, context: &GhostContext) {
        let mut sessions = self.active_sessions.write().await;
        let now = chrono::Utc::now();

        match sessions.get_mut(&context.session_id) {
            Some(session) => {
                session.last_activity = now;
                session.request_count += 1;
            }
            None => {
                let session = GhostSession {
                    context: context.clone(),
                    created_at: now,
                    last_activity: now,
                    request_count: 1,
                    total_cost: 0.0,
                };
                sessions.insert(context.session_id, session);
            }
        }
    }

    async fn enhance_request_with_service_config(&self, mut request: GhostRequest) -> Result<GhostRequest> {
        let service_name = request.context.service.as_str();

        if let Some(config) = self.service_configs.get(service_name) {
            // Apply default routing hints if none provided
            if request.routing_hints.is_none() {
                request.routing_hints = Some(config.default_routing_hints.clone());
            }

            // Apply default fallback strategy if none provided
            if request.fallback_strategy.is_none() {
                request.fallback_strategy = Some(config.fallback_strategy.clone());
            }

            // Enhance OMEN config based on Ghost service requirements
            if request.chat_request.omen.is_none() {
                let mut omen_config = OmenConfig::default();

                // Configure strategy based on service type
                match request.context.service {
                    GhostService::GhostFlow => {
                        omen_config.strategy = Some("race".to_string());
                        omen_config.k = Some(2);
                        omen_config.max_latency_ms = Some(1500);
                    }
                    GhostService::GhostLLM => {
                        omen_config.strategy = Some("speculate_k".to_string());
                        omen_config.k = Some(3);
                        omen_config.max_latency_ms = Some(3000);
                    }
                    GhostService::Zeke => {
                        omen_config.strategy = Some("race".to_string());
                        omen_config.k = Some(2);
                        omen_config.max_latency_ms = Some(2000);
                    }
                    GhostService::Jarvis => {
                        omen_config.strategy = Some("parallel_merge".to_string());
                        omen_config.k = Some(2);
                        omen_config.max_latency_ms = Some(2500);
                    }
                    _ => {
                        omen_config.strategy = Some("single".to_string());
                        omen_config.k = Some(1);
                    }
                }

                request.chat_request.omen = Some(omen_config);
            }
        }

        Ok(request)
    }

    fn create_request_context(&self, ghost_context: &GhostContext) -> RequestContext {
        let mut tags = HashMap::new();
        tags.insert("ghost_service".to_string(), ghost_context.service.as_str().to_string());
        tags.insert("session_id".to_string(), ghost_context.session_id.to_string());
        tags.insert("priority".to_string(), ghost_context.priority.to_string());

        if let Some(workflow_id) = ghost_context.workflow_id {
            tags.insert("workflow_id".to_string(), workflow_id.to_string());
        }

        if let Some(chain_id) = ghost_context.chain_id {
            tags.insert("chain_id".to_string(), chain_id.to_string());
        }

        // Add custom metadata
        for (key, value) in &ghost_context.metadata {
            tags.insert(format!("meta_{}", key), value.clone());
        }

        RequestContext {
            request_id: Uuid::new_v4(),
            user_id: Some(ghost_context.user_id.clone()),
            api_key: None,
            intent: Some("ghost_ai".to_string()),
            tags,
        }
    }

    fn estimate_tokens(&self, request: &ChatCompletionRequest) -> u32 {
        // Token estimation including vision support
        let mut total_tokens = 0u32;

        for message in &request.messages {
            // Text content
            total_tokens += (message.content.text().len() / 4) as u32;

            // Vision tokens for multimodal content
            if let MessageContent::Parts(parts) = &message.content {
                for part in parts {
                    if let ContentPart::ImageUrl { image_url } = part {
                        let image_tokens = match image_url.detail.as_deref() {
                            Some("low") => 85,
                            Some("high") => 765,
                            _ => 425,
                        };
                        total_tokens += image_tokens;
                    }
                }
            }
        }

        total_tokens
    }

    fn estimate_cost(&self, response: &ChatCompletionResponse, service_name: &str) -> f64 {
        // Ghost AI services get cost optimization
        let base_cost = (response.usage.total_tokens as f64 / 1000.0) * 0.02;

        match service_name {
            "ghostflow" => base_cost * 0.5,  // 50% discount for automation
            "ghostllm" => base_cost * 0.8,   // 20% discount for main interface
            "zeke" => base_cost * 0.7,       // 30% discount for core assistant
            "jarvis" => base_cost * 0.6,     // 40% discount for orchestration
            _ => base_cost
        }
    }

    pub async fn get_ghost_session_stats(&self, session_id: Uuid) -> Option<GhostSessionStats> {
        let sessions = self.active_sessions.read().await;
        sessions.get(&session_id).map(|session| {
            GhostSessionStats {
                session_id,
                service: session.context.service.clone(),
                user_id: session.context.user_id.clone(),
                created_at: session.created_at,
                last_activity: session.last_activity,
                request_count: session.request_count,
                total_cost: session.total_cost,
                is_active: session.last_activity > chrono::Utc::now() - chrono::Duration::minutes(30),
            }
        })
    }

    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.active_sessions.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(2);

        let initial_count = sessions.len();
        sessions.retain(|_, session| session.last_activity > cutoff);
        let final_count = sessions.len();

        if initial_count != final_count {
            info!("🧹 Cleaned up {} expired Ghost AI sessions ({} -> {})",
                  initial_count - final_count, initial_count, final_count);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GhostSessionStats {
    pub session_id: Uuid,
    pub service: GhostService,
    pub user_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub request_count: u32,
    pub total_cost: f64,
    pub is_active: bool,
}

// Helper functions for Ghost AI ecosystem integration
impl GhostOrchestrator {
    /// Create a GhostLLM optimized request
    pub fn create_ghostllm_request(user_id: String, messages: Vec<ChatMessage>) -> GhostRequest {
        let context = GhostContext::new(GhostService::GhostLLM, user_id);
        let chat_request = ChatCompletionRequest {
            model: "auto".to_string(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2000),
            stream: false,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            tools: None,
            tool_choice: None,
            omen: None,
            tags: None,
        };

        GhostRequest {
            context,
            chat_request,
            routing_hints: Some(GhostRoutingHints {
                prefer_local: false,
                max_latency_ms: Some(3000),
                cost_sensitivity: 0.3,
                quality_threshold: 0.8,
                provider_whitelist: None,
                provider_blacklist: None,
            }),
            fallback_strategy: Some(GhostFallbackStrategy::BestEffort),
        }
    }

    /// Create a GhostFlow automation request
    pub fn create_ghostflow_request(
        user_id: String,
        workflow_id: Uuid,
        messages: Vec<ChatMessage>
    ) -> GhostRequest {
        let context = GhostContext::new(GhostService::GhostFlow, user_id)
            .with_workflow(workflow_id);

        let chat_request = ChatCompletionRequest {
            model: "auto".to_string(),
            messages,
            temperature: Some(0.3), // Lower temperature for automation consistency
            max_tokens: Some(1000),
            stream: false,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            tools: None,
            tool_choice: None,
            omen: None,
            tags: None,
        };

        GhostRequest {
            context,
            chat_request,
            routing_hints: Some(GhostRoutingHints {
                prefer_local: true,
                max_latency_ms: Some(1500),
                cost_sensitivity: 0.8,
                quality_threshold: 0.6,
                provider_whitelist: None,
                provider_blacklist: None,
            }),
            fallback_strategy: Some(GhostFallbackStrategy::CheaperProvider),
        }
    }
}