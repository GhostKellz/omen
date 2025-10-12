use crate::{
    auth::UsageTracker,
    error::{OmenError, Result},
};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingTier {
    pub name: String,
    pub requests_per_day: Option<u32>,    // None = unlimited
    pub tokens_per_day: Option<u32>,      // None = unlimited
    pub budget_per_day_usd: Option<f64>,  // None = unlimited
    pub cost_multiplier: f64,             // 1.0 = standard pricing, 0.8 = 20% discount
    pub priority_weight: f64,             // Higher = better provider selection
}

impl Default for BillingTier {
    fn default() -> Self {
        Self {
            name: "free".to_string(),
            requests_per_day: Some(100),
            tokens_per_day: Some(10000),
            budget_per_day_usd: Some(1.0),
            cost_multiplier: 1.0,
            priority_weight: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBilling {
    pub user_id: String,
    pub tier: BillingTier,
    pub usage_tracker: UsageTracker,
    pub monthly_spend_usd: f64,
    pub total_spend_usd: f64,
    pub subscription_start: DateTime<Utc>,
    pub last_billing_date: DateTime<Utc>,
    pub payment_method_id: Option<String>,
    pub billing_email: Option<String>,
}

/// UserBilling implementation - all public methods are part of the API
#[allow(dead_code)]
impl UserBilling {
    pub fn new(user_id: String, tier: BillingTier) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            tier,
            usage_tracker: UsageTracker::default(),
            monthly_spend_usd: 0.0,
            total_spend_usd: 0.0,
            subscription_start: now,
            last_billing_date: now,
            payment_method_id: None,
            billing_email: None,
        }
    }

    pub fn can_make_request(&self) -> bool {
        // Check daily request limit
        if let Some(daily_limit) = self.tier.requests_per_day {
            if self.usage_tracker.requests_today >= daily_limit {
                return false;
            }
        }

        // Check daily token limit
        if let Some(token_limit) = self.tier.tokens_per_day {
            if self.usage_tracker.tokens_today >= token_limit {
                return false;
            }
        }

        // Check daily budget limit
        if let Some(budget_limit) = self.tier.budget_per_day_usd {
            if self.usage_tracker.cost_today_usd >= budget_limit {
                return false;
            }
        }

        true
    }

    pub fn estimate_cost(&self, estimated_tokens: u32, provider_cost_per_1k: f64) -> f64 {
        let base_cost = (estimated_tokens as f64 / 1000.0) * provider_cost_per_1k;
        base_cost * self.tier.cost_multiplier
    }

    pub fn record_usage(&mut self, tokens_used: u32, provider_cost_usd: f64) {
        let actual_cost = provider_cost_usd * self.tier.cost_multiplier;
        self.usage_tracker.add_usage(tokens_used, actual_cost);
        self.monthly_spend_usd += actual_cost;
        self.total_spend_usd += actual_cost;

        debug!("Recorded usage for {}: {} tokens, ${:.4} cost",
               self.user_id, tokens_used, actual_cost);
    }

    pub fn should_reset_monthly(&self) -> bool {
        let now = Utc::now();
        let current_month = (now.year(), now.month());
        let last_billing_month = (self.last_billing_date.year(), self.last_billing_date.month());
        current_month != last_billing_month
    }

    pub fn reset_monthly(&mut self) {
        if self.should_reset_monthly() {
            info!("Resetting monthly billing for user {}: ${:.2} spent last month",
                  self.user_id, self.monthly_spend_usd);
            self.monthly_spend_usd = 0.0;
            self.last_billing_date = Utc::now();
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub provider_cost_usd: f64,
    pub timestamp: DateTime<Utc>,
}

impl TokenUsage {
    pub fn new(input_tokens: u32, output_tokens: u32, provider_cost_usd: f64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            provider_cost_usd,
            timestamp: Utc::now(),
        }
    }
}

/// Billing manager - all public methods are part of the API
#[allow(dead_code)]
#[derive(Debug)]
pub struct BillingManager {
    user_billing: Arc<RwLock<HashMap<String, UserBilling>>>,
    tier_configs: HashMap<String, BillingTier>,
    usage_history: Arc<RwLock<HashMap<String, Vec<TokenUsage>>>>,
}

#[allow(dead_code)]
impl BillingManager {
    pub fn new() -> Self {
        let mut tier_configs = HashMap::new();

        // Free tier
        tier_configs.insert("free".to_string(), BillingTier {
            name: "free".to_string(),
            requests_per_day: Some(100),
            tokens_per_day: Some(10000),
            budget_per_day_usd: Some(1.0),
            cost_multiplier: 1.0,
            priority_weight: 1.0,
        });

        // Pro tier
        tier_configs.insert("pro".to_string(), BillingTier {
            name: "pro".to_string(),
            requests_per_day: Some(10000),
            tokens_per_day: Some(1000000),
            budget_per_day_usd: Some(50.0),
            cost_multiplier: 0.8, // 20% discount
            priority_weight: 1.5,
        });

        // Enterprise tier
        tier_configs.insert("enterprise".to_string(), BillingTier {
            name: "enterprise".to_string(),
            requests_per_day: None, // unlimited
            tokens_per_day: None,   // unlimited
            budget_per_day_usd: None, // unlimited
            cost_multiplier: 0.6, // 40% discount
            priority_weight: 2.0,
        });

        Self {
            user_billing: Arc::new(RwLock::new(HashMap::new())),
            tier_configs,
            usage_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_or_create_user_billing(&self, user_id: &str) -> UserBilling {
        let mut billing_map = self.user_billing.write().await;

        billing_map.entry(user_id.to_string())
            .or_insert_with(|| {
                let tier = self.tier_configs.get("free").unwrap().clone();
                UserBilling::new(user_id.to_string(), tier)
            })
            .clone()
    }

    pub async fn update_user_tier(&self, user_id: &str, tier_name: &str) -> Result<()> {
        let tier = self.tier_configs.get(tier_name)
            .ok_or_else(|| OmenError::InvalidRequest(format!("Unknown tier: {}", tier_name)))?
            .clone();

        let mut billing_map = self.user_billing.write().await;
        if let Some(user_billing) = billing_map.get_mut(user_id) {
            user_billing.tier = tier;
            info!("Updated user {} to tier {}", user_id, tier_name);
        } else {
            let user_billing = UserBilling::new(user_id.to_string(), tier);
            billing_map.insert(user_id.to_string(), user_billing);
            info!("Created new user {} with tier {}", user_id, tier_name);
        }

        Ok(())
    }

    pub async fn check_request_allowed(&self, user_id: &str) -> Result<bool> {
        let user_billing = self.get_or_create_user_billing(user_id).await;
        Ok(user_billing.can_make_request())
    }

    pub async fn estimate_request_cost(
        &self,
        user_id: &str,
        estimated_tokens: u32,
        provider_cost_per_1k: f64
    ) -> Result<f64> {
        let user_billing = self.get_or_create_user_billing(user_id).await;
        Ok(user_billing.estimate_cost(estimated_tokens, provider_cost_per_1k))
    }

    pub async fn record_usage(
        &self,
        user_id: &str,
        input_tokens: u32,
        output_tokens: u32,
        provider_cost_usd: f64,
    ) -> Result<()> {
        // Update user billing
        {
            let mut billing_map = self.user_billing.write().await;
            if let Some(user_billing) = billing_map.get_mut(user_id) {
                user_billing.reset_monthly(); // Check if we need monthly reset
                user_billing.record_usage(input_tokens + output_tokens, provider_cost_usd);
            }
        }

        // Record usage history
        let usage = TokenUsage::new(input_tokens, output_tokens, provider_cost_usd);
        let mut history = self.usage_history.write().await;
        history.entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(usage);

        Ok(())
    }

    pub async fn get_user_usage_stats(&self, user_id: &str) -> Result<UserUsageStats> {
        let user_billing = self.get_or_create_user_billing(user_id).await;
        let history = self.usage_history.read().await;

        let usage_history = history.get(user_id).cloned().unwrap_or_default();

        // Calculate monthly stats
        let now = Utc::now();
        let monthly_usage: Vec<_> = usage_history.iter()
            .filter(|u| {
                let usage_month = (u.timestamp.year(), u.timestamp.month());
                let current_month = (now.year(), now.month());
                usage_month == current_month
            })
            .collect();

        let monthly_tokens: u32 = monthly_usage.iter().map(|u| u.total_tokens).sum();
        let monthly_cost: f64 = monthly_usage.iter().map(|u| u.provider_cost_usd).sum();

        Ok(UserUsageStats {
            user_id: user_id.to_string(),
            tier: user_billing.tier.name.clone(),
            daily_requests: user_billing.usage_tracker.requests_today,
            daily_tokens: user_billing.usage_tracker.tokens_today,
            daily_cost_usd: user_billing.usage_tracker.cost_today_usd,
            monthly_tokens,
            monthly_cost_usd: monthly_cost,
            total_cost_usd: user_billing.total_spend_usd,
            daily_limits: UserLimits {
                requests: user_billing.tier.requests_per_day,
                tokens: user_billing.tier.tokens_per_day,
                budget_usd: user_billing.tier.budget_per_day_usd,
            },
            can_make_request: user_billing.can_make_request(),
        })
    }

    pub async fn get_all_users_summary(&self) -> Vec<UserBillingSummary> {
        let billing_map = self.user_billing.read().await;

        billing_map.values()
            .map(|billing| UserBillingSummary {
                user_id: billing.user_id.clone(),
                tier: billing.tier.name.clone(),
                daily_cost_usd: billing.usage_tracker.cost_today_usd,
                monthly_spend_usd: billing.monthly_spend_usd,
                total_spend_usd: billing.total_spend_usd,
                last_activity: billing.usage_tracker.last_reset,
            })
            .collect()
    }

    pub fn get_tier_config(&self, tier_name: &str) -> Option<&BillingTier> {
        self.tier_configs.get(tier_name)
    }

    pub fn list_available_tiers(&self) -> Vec<&BillingTier> {
        self.tier_configs.values().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUsageStats {
    pub user_id: String,
    pub tier: String,
    pub daily_requests: u32,
    pub daily_tokens: u32,
    pub daily_cost_usd: f64,
    pub monthly_tokens: u32,
    pub monthly_cost_usd: f64,
    pub total_cost_usd: f64,
    pub daily_limits: UserLimits,
    pub can_make_request: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLimits {
    pub requests: Option<u32>,
    pub tokens: Option<u32>,
    pub budget_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBillingSummary {
    pub user_id: String,
    pub tier: String,
    pub daily_cost_usd: f64,
    pub monthly_spend_usd: f64,
    pub total_spend_usd: f64,
    pub last_activity: DateTime<Utc>,
}

impl Default for BillingManager {
    fn default() -> Self {
        Self::new()
    }
}