use crate::{
    billing::BillingManager,
    error::{OmenError, Result},
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub burst_allowance: u32,
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            tokens_per_minute: 10000,
            burst_allowance: 10,
            window_size: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitBucket {
    pub requests: u32,
    pub tokens: u32,
    pub last_refill: Instant,
    pub window_start: Instant,
}

impl RateLimitBucket {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            requests: 0,
            tokens: 0,
            last_refill: now,
            window_start: now,
        }
    }

    pub fn should_reset(&self, window_size: Duration) -> bool {
        self.window_start.elapsed() >= window_size
    }

    pub fn reset(&mut self) {
        let now = Instant::now();
        self.requests = 0;
        self.tokens = 0;
        self.last_refill = now;
        self.window_start = now;
    }

    pub fn can_consume(&self, config: &RateLimitConfig, tokens_requested: u32) -> bool {
        self.requests < config.requests_per_minute + config.burst_allowance &&
        self.tokens + tokens_requested <= config.tokens_per_minute
    }

    pub fn consume(&mut self, tokens: u32) {
        self.requests += 1;
        self.tokens += tokens;
    }
}

#[derive(Debug)]
pub struct AdaptiveRateLimiter {
    buckets: Arc<RwLock<HashMap<String, RateLimitBucket>>>,
    tier_configs: HashMap<String, RateLimitConfig>,
    billing_manager: Arc<BillingManager>,
}

impl AdaptiveRateLimiter {
    pub fn new(billing_manager: Arc<BillingManager>) -> Self {
        let mut tier_configs = HashMap::new();

        // Free tier - conservative limits
        tier_configs.insert("free".to_string(), RateLimitConfig {
            requests_per_minute: 20,
            tokens_per_minute: 2000,
            burst_allowance: 5,
            window_size: Duration::from_secs(60),
        });

        // Pro tier - generous limits
        tier_configs.insert("pro".to_string(), RateLimitConfig {
            requests_per_minute: 200,
            tokens_per_minute: 50000,
            burst_allowance: 20,
            window_size: Duration::from_secs(60),
        });

        // Enterprise tier - very high limits
        tier_configs.insert("enterprise".to_string(), RateLimitConfig {
            requests_per_minute: 1000,
            tokens_per_minute: 500000,
            burst_allowance: 100,
            window_size: Duration::from_secs(60),
        });

        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            tier_configs,
            billing_manager,
        }
    }

    pub async fn check_rate_limit(&self, user_id: &str, estimated_tokens: u32) -> Result<()> {
        // Get user billing info to determine tier
        let user_billing = self.billing_manager.get_or_create_user_billing(user_id).await;
        let tier_name = &user_billing.tier.name;

        let config = self.tier_configs.get(tier_name)
            .unwrap_or_else(|| self.tier_configs.get("free").unwrap());

        let mut buckets = self.buckets.write().await;
        let bucket = buckets.entry(user_id.to_string())
            .or_insert_with(RateLimitBucket::new);

        // Reset bucket if window expired
        if bucket.should_reset(config.window_size) {
            bucket.reset();
            debug!("Reset rate limit bucket for user: {}", user_id);
        }

        // Check if request can be allowed
        if !bucket.can_consume(config, estimated_tokens) {
            warn!("Rate limit exceeded for user {}: {} req/min, {} tokens/min",
                  user_id, bucket.requests, bucket.tokens);
            return Err(OmenError::RateLimitExceeded);
        }

        // Consume the quota
        bucket.consume(estimated_tokens);

        debug!("Rate limit check passed for user {}: {}/{} requests, {}/{} tokens",
               user_id, bucket.requests, config.requests_per_minute,
               bucket.tokens, config.tokens_per_minute);

        Ok(())
    }

    pub async fn get_rate_limit_status(&self, user_id: &str) -> RateLimitStatus {
        let user_billing = self.billing_manager.get_or_create_user_billing(user_id).await;
        let tier_name = &user_billing.tier.name;

        let config = self.tier_configs.get(tier_name)
            .unwrap_or_else(|| self.tier_configs.get("free").unwrap());

        let buckets = self.buckets.read().await;
        let bucket = buckets.get(user_id);

        match bucket {
            Some(bucket) if !bucket.should_reset(config.window_size) => {
                RateLimitStatus {
                    tier: tier_name.clone(),
                    requests_used: bucket.requests,
                    requests_limit: config.requests_per_minute,
                    tokens_used: bucket.tokens,
                    tokens_limit: config.tokens_per_minute,
                    window_reset_in_seconds: config.window_size.as_secs() - bucket.window_start.elapsed().as_secs(),
                    burst_available: config.burst_allowance.saturating_sub(bucket.requests.saturating_sub(config.requests_per_minute)),
                }
            }
            _ => {
                // No bucket or expired - user is at full capacity
                RateLimitStatus {
                    tier: tier_name.clone(),
                    requests_used: 0,
                    requests_limit: config.requests_per_minute,
                    tokens_used: 0,
                    tokens_limit: config.tokens_per_minute,
                    window_reset_in_seconds: config.window_size.as_secs(),
                    burst_available: config.burst_allowance,
                }
            }
        }
    }

    pub async fn cleanup_expired_buckets(&self) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();

        buckets.retain(|_, bucket| {
            // Keep buckets that are less than 2 windows old
            now.duration_since(bucket.window_start) < Duration::from_secs(120)
        });

        debug!("Cleaned up expired rate limit buckets. Active buckets: {}", buckets.len());
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitStatus {
    pub tier: String,
    pub requests_used: u32,
    pub requests_limit: u32,
    pub tokens_used: u32,
    pub tokens_limit: u32,
    pub window_reset_in_seconds: u64,
    pub burst_available: u32,
}

// Ghost AI specific rate limiting for inter-service communication
#[derive(Debug)]
pub struct GhostAIRateLimiter {
    adaptive_limiter: AdaptiveRateLimiter,
    service_priorities: HashMap<String, u8>, // 0-255, higher = more priority
}

#[allow(dead_code)]
impl GhostAIRateLimiter {
    pub fn new(billing_manager: Arc<BillingManager>) -> Self {
        let adaptive_limiter = AdaptiveRateLimiter::new(billing_manager);

        let mut service_priorities = HashMap::new();
        // Ghost AI service priorities
        service_priorities.insert("ghostllm".to_string(), 255);      // Highest - main interface
        service_priorities.insert("ghostflow".to_string(), 200);     // High - automation
        service_priorities.insert("zeke".to_string(), 180);          // High - core assistant
        service_priorities.insert("jarvis".to_string(), 160);        // Medium-high - orchestration
        service_priorities.insert("external".to_string(), 100);      // Medium - external users
        service_priorities.insert("development".to_string(), 50);    // Low - dev/testing

        Self {
            adaptive_limiter,
            service_priorities,
        }
    }

    pub async fn check_ghost_service_limit(
        &self,
        service_name: &str,
        user_id: &str,
        estimated_tokens: u32
    ) -> Result<()> {
        // Get service priority
        let priority = self.service_priorities.get(service_name).unwrap_or(&100);

        // High priority services get enhanced limits
        if *priority >= 200 {
            // Ghost core services - apply relaxed rate limiting
            self.check_enhanced_rate_limit(user_id, estimated_tokens, *priority).await
        } else {
            // Standard rate limiting for external/lower priority services
            self.adaptive_limiter.check_rate_limit(user_id, estimated_tokens).await
        }
    }

    async fn check_enhanced_rate_limit(
        &self,
        user_id: &str,
        estimated_tokens: u32,
        priority: u8
    ) -> Result<()> {
        // Enhanced limits for Ghost AI services based on priority
        let _multiplier = match priority {
            255 => 5.0,  // GhostLLM gets 5x normal limits
            200 => 3.0,  // GhostFlow gets 3x normal limits
            180 => 2.5,  // Zeke gets 2.5x normal limits
            160 => 2.0,  // Jarvis gets 2x normal limits
            _ => 1.0,    // Standard limits
        };

        // TODO: Implement enhanced rate limiting logic (use _multiplier)
        // For now, just use standard rate limiting
        self.adaptive_limiter.check_rate_limit(user_id, estimated_tokens).await
    }

    pub async fn get_ghost_service_status(&self, service_name: &str, user_id: &str) -> GhostServiceStatus {
        let standard_status = self.adaptive_limiter.get_rate_limit_status(user_id).await;
        let priority = self.service_priorities.get(service_name).unwrap_or(&100);

        GhostServiceStatus {
            service_name: service_name.to_string(),
            priority: *priority,
            standard_limits: standard_status,
            enhanced_multiplier: match priority {
                255 => 5.0,
                200 => 3.0,
                180 => 2.5,
                160 => 2.0,
                _ => 1.0,
            },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GhostServiceStatus {
    pub service_name: String,
    pub priority: u8,
    pub standard_limits: RateLimitStatus,
    pub enhanced_multiplier: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limit_bucket() {
        let mut bucket = RateLimitBucket::new();
        let config = RateLimitConfig::default();

        // Should allow initial requests
        assert!(bucket.can_consume(&config, 100));
        bucket.consume(100);

        // Should track usage
        assert_eq!(bucket.requests, 1);
        assert_eq!(bucket.tokens, 100);

        // Should deny when over limit
        bucket.tokens = config.tokens_per_minute;
        assert!(!bucket.can_consume(&config, 1));
    }

    #[tokio::test]
    async fn test_bucket_reset() {
        let mut bucket = RateLimitBucket::new();
        let config = RateLimitConfig {
            window_size: Duration::from_millis(100),
            ..Default::default()
        };

        bucket.consume(1000);
        assert_eq!(bucket.requests, 1);

        // Wait for window to expire
        sleep(Duration::from_millis(150)).await;
        assert!(bucket.should_reset(config.window_size));

        bucket.reset();
        assert_eq!(bucket.requests, 0);
        assert_eq!(bucket.tokens, 0);
    }
}