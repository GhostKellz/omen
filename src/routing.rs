use crate::{
    error::Result,
    providers::Provider,
    types::*,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingStrategy {
    pub cost_weight: f32,      // 0.0-1.0, higher = more cost sensitive
    pub latency_weight: f32,   // 0.0-1.0, higher = more latency sensitive
    pub quality_weight: f32,   // 0.0-1.0, higher = more quality sensitive
    pub reliability_weight: f32, // 0.0-1.0, higher = more reliability sensitive
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self {
            cost_weight: 0.3,
            latency_weight: 0.4,
            quality_weight: 0.2,
            reliability_weight: 0.1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderMetrics {
    pub provider_id: String,
    pub avg_latency_ms: f64,
    pub success_rate: f64,        // 0.0-1.0
    pub cost_per_1k_tokens: f64,
    pub quality_score: f64,       // 0.0-1.0, based on user feedback/model capabilities
    pub current_load: f64,        // 0.0-1.0, current utilization
    pub availability: f64,        // 0.0-1.0, uptime percentage
}

impl Default for ProviderMetrics {
    fn default() -> Self {
        Self {
            provider_id: String::new(),
            avg_latency_ms: 1000.0,
            success_rate: 0.95,
            cost_per_1k_tokens: 0.01,
            quality_score: 0.8,
            current_load: 0.5,
            availability: 0.99,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RoutingDecision {
    pub selected_providers: Vec<String>,
    pub strategy_used: String,
    pub estimated_cost: f64,
    pub estimated_latency_ms: u64,
    pub confidence_score: f64,
    pub reasoning: Vec<String>,
}

#[derive(Debug)]
pub struct AdvancedRouter {
    metrics: HashMap<String, ProviderMetrics>,
    strategy: RoutingStrategy,
    cost_budgets: HashMap<String, f64>, // per-user budget tracking
    latency_targets: HashMap<String, u64>, // per-intent SLA targets
}

/// AdvancedRouter implementation - all public methods are part of the routing API
#[allow(dead_code)]
impl AdvancedRouter {
    pub fn new() -> Self {
        let mut latency_targets = HashMap::new();
        latency_targets.insert("code".to_string(), 2000); // 2s for code generation
        latency_targets.insert("tests".to_string(), 3000); // 3s for test generation
        latency_targets.insert("analysis".to_string(), 5000); // 5s for analysis
        latency_targets.insert("general".to_string(), 3000); // 3s default

        Self {
            metrics: HashMap::new(),
            strategy: RoutingStrategy::default(),
            cost_budgets: HashMap::new(),
            latency_targets,
        }
    }

    pub fn update_provider_metrics(&mut self, provider_id: &str, metrics: ProviderMetrics) {
        self.metrics.insert(provider_id.to_string(), metrics);
    }

    pub fn set_strategy(&mut self, strategy: RoutingStrategy) {
        self.strategy = strategy;
    }

    pub fn set_user_budget(&mut self, user_id: &str, budget_usd: f64) {
        self.cost_budgets.insert(user_id.to_string(), budget_usd);
    }

    pub async fn select_optimal_providers(
        &self,
        providers: &[Arc<dyn Provider>],
        request: &ChatCompletionRequest,
        context: &RequestContext,
        k: usize,
    ) -> Result<RoutingDecision> {
        let mut provider_scores = Vec::new();
        let intent = context.intent.as_deref().unwrap_or("general");
        let target_latency = self.latency_targets.get(intent).unwrap_or(&3000);

        // Calculate score for each provider
        for provider in providers {
            let provider_id = provider.id();
            let metrics = self.metrics.get(provider_id)
                .cloned()
                .unwrap_or_else(|| {
                    let mut default_metrics = ProviderMetrics::default();
                    default_metrics.provider_id = provider_id.to_string();
                    self.get_default_metrics_for_provider(provider_id, default_metrics)
                });

            let score = self.calculate_provider_score(&metrics, *target_latency, intent);
            provider_scores.push((provider.clone(), metrics, score));
        }

        // Sort by score (higher is better)
        provider_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Select top k providers
        let selected_count = k.min(provider_scores.len());
        let selected: Vec<_> = provider_scores.iter().take(selected_count).collect();

        let selected_providers = selected.iter().map(|(p, _, _)| p.id().to_string()).collect();
        let estimated_cost = self.estimate_total_cost(&selected, request);
        let estimated_latency = self.estimate_response_latency(&selected);
        let confidence_score = self.calculate_confidence_score(&selected);

        let reasoning = self.generate_reasoning(&selected, intent, *target_latency);

        info!("🎯 Advanced routing selected {} providers for intent '{}': {:?}",
              selected_count, intent, selected_providers);

        Ok(RoutingDecision {
            selected_providers,
            strategy_used: format!("advanced_weighted_{}", intent),
            estimated_cost,
            estimated_latency_ms: estimated_latency,
            confidence_score,
            reasoning,
        })
    }

    fn calculate_provider_score(&self, metrics: &ProviderMetrics, target_latency: u64, intent: &str) -> f64 {
        // Normalize metrics to 0-1 range
        let latency_score = self.calculate_latency_score(metrics.avg_latency_ms, target_latency as f64);
        let cost_score = self.calculate_cost_score(metrics.cost_per_1k_tokens);
        let quality_score = metrics.quality_score;
        let reliability_score = metrics.success_rate * metrics.availability;

        // Apply intent-specific weights
        let (cost_w, latency_w, quality_w, reliability_w) = self.get_intent_weights(intent);

        let total_score = (cost_score * cost_w as f64) +
                         (latency_score * latency_w as f64) +
                         (quality_score * quality_w as f64) +
                         (reliability_score * reliability_w as f64);

        // Apply load balancing penalty
        let load_penalty = 1.0 - (metrics.current_load * 0.2); // Max 20% penalty for high load

        debug!("Provider {} scores: latency={:.3}, cost={:.3}, quality={:.3}, reliability={:.3}, total={:.3}",
               metrics.provider_id, latency_score, cost_score, quality_score, reliability_score, total_score);

        total_score * load_penalty
    }

    fn calculate_latency_score(&self, avg_latency_ms: f64, target_latency_ms: f64) -> f64 {
        // Score decreases as latency increases beyond target
        if avg_latency_ms <= target_latency_ms {
            1.0 - (avg_latency_ms / target_latency_ms * 0.2) // Small penalty even for good latency
        } else {
            let penalty = (avg_latency_ms - target_latency_ms) / target_latency_ms;
            (1.0 - penalty.min(1.0)).max(0.0)
        }
    }

    fn calculate_cost_score(&self, cost_per_1k: f64) -> f64 {
        // Normalize cost (assuming $0.10 per 1k tokens as expensive)
        let max_reasonable_cost = 0.1;
        (1.0 - (cost_per_1k / max_reasonable_cost).min(1.0)).max(0.0)
    }

    fn get_intent_weights(&self, intent: &str) -> (f32, f32, f32, f32) {
        match intent {
            "code" => (0.2, 0.5, 0.2, 0.1),      // Latency-focused for coding
            "tests" => (0.3, 0.4, 0.2, 0.1),     // Balanced for test generation
            "analysis" => (0.2, 0.3, 0.4, 0.1),  // Quality-focused for analysis
            "explanation" => (0.4, 0.2, 0.3, 0.1), // Cost-focused for explanations
            "regex" => (0.2, 0.6, 0.1, 0.1),     // Very latency-focused
            _ => (self.strategy.cost_weight, self.strategy.latency_weight,
                  self.strategy.quality_weight, self.strategy.reliability_weight),
        }
    }

    fn get_default_metrics_for_provider(&self, provider_id: &str, mut metrics: ProviderMetrics) -> ProviderMetrics {
        // Set provider-specific defaults based on known characteristics
        match provider_id {
            "ollama" => {
                metrics.avg_latency_ms = 500.0;   // Fast local
                metrics.cost_per_1k_tokens = 0.0; // Free local
                metrics.quality_score = 0.7;      // Good but not top-tier
                metrics.availability = 0.95;      // Local availability
            }
            "openai" => {
                metrics.avg_latency_ms = 1500.0;
                metrics.cost_per_1k_tokens = 0.03; // GPT-4 pricing
                metrics.quality_score = 0.9;
                metrics.availability = 0.99;
            }
            "anthropic" => {
                metrics.avg_latency_ms = 1200.0;
                metrics.cost_per_1k_tokens = 0.015; // Claude pricing
                metrics.quality_score = 0.95;
                metrics.availability = 0.98;
            }
            "google" => {
                metrics.avg_latency_ms = 1000.0;
                metrics.cost_per_1k_tokens = 0.00125; // Gemini pricing
                metrics.quality_score = 0.85;
                metrics.availability = 0.97;
            }
            "azure" => {
                metrics.avg_latency_ms = 1800.0;
                metrics.cost_per_1k_tokens = 0.03;
                metrics.quality_score = 0.9;
                metrics.availability = 0.99;
            }
            "xai" => {
                metrics.avg_latency_ms = 1300.0;
                metrics.cost_per_1k_tokens = 0.0; // Free during beta
                metrics.quality_score = 0.8;
                metrics.availability = 0.95;
            }
            "bedrock" => {
                metrics.avg_latency_ms = 2000.0;
                metrics.cost_per_1k_tokens = 0.015;
                metrics.quality_score = 0.9;
                metrics.availability = 0.99;
            }
            _ => {} // Keep defaults
        }
        metrics
    }

    fn estimate_total_cost(&self, selected: &[&(Arc<dyn Provider>, ProviderMetrics, f64)], request: &ChatCompletionRequest) -> f64 {
        // Estimate token count (rough approximation)
        let estimated_input_tokens = request.messages.iter()
            .map(|m| m.content.len() / 4) // Rough token estimation
            .sum::<usize>() as f64;

        let estimated_output_tokens = request.max_tokens.unwrap_or(500) as f64;

        // For race strategy, we pay for the winner + partial cost for losers
        let mut total_cost = 0.0;
        for (i, (_, metrics, _)) in selected.iter().enumerate() {
            let input_cost = (estimated_input_tokens / 1000.0) * metrics.cost_per_1k_tokens;
            let output_cost = if i == 0 { // Winner gets full output cost
                (estimated_output_tokens / 1000.0) * metrics.cost_per_1k_tokens
            } else { // Losers get partial output cost (early cancellation)
                (estimated_output_tokens * 0.2 / 1000.0) * metrics.cost_per_1k_tokens
            };
            total_cost += input_cost + output_cost;
        }

        total_cost
    }

    fn estimate_response_latency(&self, selected: &[&(Arc<dyn Provider>, ProviderMetrics, f64)]) -> u64 {
        // For race strategy, latency is the minimum expected latency
        selected.iter()
            .map(|(_, metrics, _)| metrics.avg_latency_ms as u64)
            .min()
            .unwrap_or(3000)
    }

    fn calculate_confidence_score(&self, selected: &[&(Arc<dyn Provider>, ProviderMetrics, f64)]) -> f64 {
        // Confidence based on reliability and diversity of selected providers
        let avg_reliability = selected.iter()
            .map(|(_, metrics, _)| metrics.success_rate * metrics.availability)
            .sum::<f64>() / selected.len() as f64;

        // Diversity bonus for having multiple providers
        let diversity_bonus = if selected.len() > 1 { 0.1 } else { 0.0 };

        (avg_reliability + diversity_bonus).min(1.0)
    }

    fn generate_reasoning(&self, selected: &[&(Arc<dyn Provider>, ProviderMetrics, f64)], intent: &str, target_latency: u64) -> Vec<String> {
        let mut reasoning = Vec::new();

        reasoning.push(format!("Selected {} providers for '{}' intent", selected.len(), intent));
        reasoning.push(format!("Target latency: {}ms", target_latency));

        for (i, (provider, metrics, score)) in selected.iter().enumerate() {
            reasoning.push(format!(
                "#{}: {} (score: {:.3}, latency: {:.0}ms, cost: ${:.4}/1k, quality: {:.2})",
                i + 1, provider.name(), score, metrics.avg_latency_ms,
                metrics.cost_per_1k_tokens, metrics.quality_score
            ));
        }

        if selected.len() > 1 {
            reasoning.push("Using race strategy for optimal latency".to_string());
        }

        reasoning
    }

    pub fn update_metrics_from_response(
        &mut self,
        provider_id: &str,
        latency_ms: u64,
        success: bool,
        cost_usd: f64,
        tokens_used: u32,
    ) {
        // Check if we need to create default metrics first
        if !self.metrics.contains_key(provider_id) {
            let mut default = ProviderMetrics::default();
            default.provider_id = provider_id.to_string();
            let default_metrics = self.get_default_metrics_for_provider(provider_id, default);
            self.metrics.insert(provider_id.to_string(), default_metrics);
        }

        // Now safely get the metrics for update
        let metrics = self.metrics.get_mut(provider_id).unwrap();

        // Update with exponential moving average
        let alpha = 0.1; // Learning rate
        metrics.avg_latency_ms = metrics.avg_latency_ms * (1.0 - alpha) + (latency_ms as f64) * alpha;

        if success {
            metrics.success_rate = metrics.success_rate * (1.0 - alpha) + alpha;
        } else {
            metrics.success_rate = metrics.success_rate * (1.0 - alpha);
        }

        // Update cost per token if we have data
        if tokens_used > 0 {
            let cost_per_1k = (cost_usd / tokens_used as f64) * 1000.0;
            metrics.cost_per_1k_tokens = metrics.cost_per_1k_tokens * (1.0 - alpha) + cost_per_1k * alpha;
        }

        debug!("Updated metrics for {}: latency={:.1}ms, success_rate={:.3}, cost_per_1k=${:.4}",
               provider_id, metrics.avg_latency_ms, metrics.success_rate, metrics.cost_per_1k_tokens);
    }
}