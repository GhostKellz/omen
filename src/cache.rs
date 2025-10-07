use crate::{
    error::{OmenError, Result},
    types::*,
};
use redis::{Client, Commands, Connection};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub redis_url: String,
    pub default_ttl_seconds: u64,
    pub response_cache_ttl: u64,
    pub session_cache_ttl: u64,
    pub rate_limit_ttl: u64,
    pub provider_health_ttl: u64,
    pub max_cache_size_mb: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            default_ttl_seconds: 3600, // 1 hour
            response_cache_ttl: 1800,   // 30 minutes for AI responses
            session_cache_ttl: 7200,    // 2 hours for Ghost sessions
            rate_limit_ttl: 60,         // 1 minute for rate limits
            provider_health_ttl: 300,   // 5 minutes for provider health
            max_cache_size_mb: 1024,    // 1GB max cache size
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub response: ChatCompletionResponse,
    pub provider_used: String,
    pub cost_usd: f64,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub cache_hit_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSession {
    pub session_id: Uuid,
    pub service: String,
    pub user_id: String,
    pub workflow_data: serde_json::Value,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub request_count: u32,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedProviderHealth {
    pub provider_id: String,
    pub healthy: bool,
    pub last_checked: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
}

pub struct RedisCache {
    client: Client,
    connection_pool: Arc<RwLock<Vec<Connection>>>,
    config: CacheConfig,
}

impl std::fmt::Debug for RedisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCache")
            .field("config", &self.config)
            .finish()
    }
}

impl RedisCache {
    pub fn new(config: CacheConfig) -> Result<Self> {
        let client = Client::open(config.redis_url.clone())
            .map_err(|e| OmenError::CacheError(format!("Failed to connect to Redis: {}", e)))?;

        info!("🔗 Connected to Redis at {}", config.redis_url);

        Ok(Self {
            client,
            connection_pool: Arc::new(RwLock::new(Vec::new())),
            config,
        })
    }

    async fn get_connection(&self) -> Result<Connection> {
        // Try to get connection from pool first
        {
            let mut pool = self.connection_pool.write().await;
            if let Some(conn) = pool.pop() {
                return Ok(conn);
            }
        }

        // Create new connection if pool is empty
        self.client.get_connection()
            .map_err(|e| OmenError::CacheError(format!("Failed to get Redis connection: {}", e)))
    }

    async fn return_connection(&self, conn: Connection) {
        let mut pool = self.connection_pool.write().await;
        if pool.len() < 10 { // Max 10 connections in pool
            pool.push(conn);
        }
    }

    // Response caching - the big cost saver
    pub async fn get_cached_response(&self, cache_key: &str) -> Result<Option<CachedResponse>> {
        let mut conn = self.get_connection().await?;

        let result: Option<String> = conn.get(format!("response:{}", cache_key))
            .map_err(|e| OmenError::CacheError(format!("Cache get failed: {}", e)))?;

        self.return_connection(conn).await;

        match result {
            Some(data) => {
                match serde_json::from_str::<CachedResponse>(&data) {
                    Ok(cached) => {
                        debug!("✨ Cache HIT for response: {}", cache_key);
                        Ok(Some(cached))
                    }
                    Err(e) => {
                        warn!("Failed to deserialize cached response: {}", e);
                        Ok(None)
                    }
                }
            }
            None => {
                debug!("💨 Cache MISS for response: {}", cache_key);
                Ok(None)
            }
        }
    }

    pub async fn cache_response(
        &self,
        cache_key: &str,
        response: &ChatCompletionResponse,
        provider_used: &str,
        cost_usd: f64,
    ) -> Result<()> {
        let cached = CachedResponse {
            response: response.clone(),
            provider_used: provider_used.to_string(),
            cost_usd,
            cached_at: chrono::Utc::now(),
            cache_hit_count: 0,
        };

        let data = serde_json::to_string(&cached)
            .map_err(|e| OmenError::CacheError(format!("Serialization failed: {}", e)))?;

        let mut conn = self.get_connection().await?;

        let _: () = conn.set_ex(
            format!("response:{}", cache_key),
            data,
            self.config.response_cache_ttl,
        ).map_err(|e| OmenError::CacheError(format!("Cache set failed: {}", e)))?;

        self.return_connection(conn).await;

        debug!("💾 Cached response: {} (TTL: {}s)", cache_key, self.config.response_cache_ttl);
        Ok(())
    }

    pub async fn increment_cache_hit(&self, cache_key: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;

        let _: () = conn.incr(format!("hits:{}", cache_key), 1)
            .map_err(|e| OmenError::CacheError(format!("Hit increment failed: {}", e)))?;

        self.return_connection(conn).await;
        Ok(())
    }

    // Ghost AI session caching
    pub async fn get_cached_session(&self, session_id: Uuid) -> Result<Option<CachedSession>> {
        let mut conn = self.get_connection().await?;

        let result: Option<String> = conn.get(format!("session:{}", session_id))
            .map_err(|e| OmenError::CacheError(format!("Session get failed: {}", e)))?;

        self.return_connection(conn).await;

        match result {
            Some(data) => {
                match serde_json::from_str::<CachedSession>(&data) {
                    Ok(session) => Ok(Some(session)),
                    Err(e) => {
                        warn!("Failed to deserialize cached session: {}", e);
                        Ok(None)
                    }
                }
            }
            None => Ok(None),
        }
    }

    pub async fn cache_session(
        &self,
        session_id: Uuid,
        service: &str,
        user_id: &str,
        workflow_data: serde_json::Value,
        request_count: u32,
        total_cost: f64,
    ) -> Result<()> {
        let session = CachedSession {
            session_id,
            service: service.to_string(),
            user_id: user_id.to_string(),
            workflow_data,
            last_activity: chrono::Utc::now(),
            request_count,
            total_cost,
        };

        let data = serde_json::to_string(&session)
            .map_err(|e| OmenError::CacheError(format!("Session serialization failed: {}", e)))?;

        let mut conn = self.get_connection().await?;

        let _: () = conn.set_ex(
            format!("session:{}", session_id),
            data,
            self.config.session_cache_ttl,
        ).map_err(|e| OmenError::CacheError(format!("Session cache set failed: {}", e)))?;

        self.return_connection(conn).await;

        debug!("🎯 Cached Ghost session: {} for {}", session_id, service);
        Ok(())
    }

    // Rate limiting cache - distributed rate limiting
    pub async fn get_rate_limit_usage(&self, user_id: &str, window: &str) -> Result<Option<u32>> {
        let mut conn = self.get_connection().await?;

        let result: Option<u32> = conn.get(format!("rate:{}:{}", user_id, window))
            .map_err(|e| OmenError::CacheError(format!("Rate limit get failed: {}", e)))?;

        self.return_connection(conn).await;
        Ok(result)
    }

    pub async fn increment_rate_limit(&self, user_id: &str, window: &str, tokens: u32) -> Result<u32> {
        let mut conn = self.get_connection().await?;

        let key = format!("rate:{}:{}", user_id, window);

        // Use atomic increment with expiry
        let new_count: u32 = conn.incr(&key, tokens)
            .map_err(|e| OmenError::CacheError(format!("Rate limit increment failed: {}", e)))?;

        // Set expiry if this is a new key
        if new_count == tokens {
            let _: () = conn.expire(&key, self.config.rate_limit_ttl as i64)
                .map_err(|e| OmenError::CacheError(format!("Rate limit expiry failed: {}", e)))?;
        }

        self.return_connection(conn).await;
        Ok(new_count)
    }

    // Provider health caching
    pub async fn get_cached_provider_health(&self, provider_id: &str) -> Result<Option<CachedProviderHealth>> {
        let mut conn = self.get_connection().await?;

        let result: Option<String> = conn.get(format!("health:{}", provider_id))
            .map_err(|e| OmenError::CacheError(format!("Provider health get failed: {}", e)))?;

        self.return_connection(conn).await;

        match result {
            Some(data) => {
                match serde_json::from_str::<CachedProviderHealth>(&data) {
                    Ok(health) => Ok(Some(health)),
                    Err(e) => {
                        warn!("Failed to deserialize cached provider health: {}", e);
                        Ok(None)
                    }
                }
            }
            None => Ok(None),
        }
    }

    pub async fn cache_provider_health(
        &self,
        provider_id: &str,
        healthy: bool,
        response_time_ms: u64,
        error_message: Option<String>,
    ) -> Result<()> {
        let health = CachedProviderHealth {
            provider_id: provider_id.to_string(),
            healthy,
            last_checked: chrono::Utc::now(),
            response_time_ms,
            error_message,
        };

        let data = serde_json::to_string(&health)
            .map_err(|e| OmenError::CacheError(format!("Health serialization failed: {}", e)))?;

        let mut conn = self.get_connection().await?;

        let _: () = conn.set_ex(
            format!("health:{}", provider_id),
            data,
            self.config.provider_health_ttl,
        ).map_err(|e| OmenError::CacheError(format!("Health cache set failed: {}", e)))?;

        self.return_connection(conn).await;

        debug!("🏥 Cached provider health: {} = {}", provider_id, healthy);
        Ok(())
    }

    // Cache analytics
    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        let mut conn = self.get_connection().await?;

        let info: String = redis::cmd("INFO").query(&mut conn)
            .map_err(|e| OmenError::CacheError(format!("Redis info failed: {}", e)))?;

        self.return_connection(conn).await;

        // Parse Redis info for memory usage, hit rates, etc.
        let memory_used = Self::extract_redis_stat(&info, "used_memory:");
        let memory_peak = Self::extract_redis_stat(&info, "used_memory_peak:");
        let keyspace_hits = Self::extract_redis_stat(&info, "keyspace_hits:");
        let keyspace_misses = Self::extract_redis_stat(&info, "keyspace_misses:");

        let hit_rate = if keyspace_hits + keyspace_misses > 0 {
            keyspace_hits as f64 / (keyspace_hits + keyspace_misses) as f64
        } else {
            0.0
        };

        Ok(CacheStats {
            memory_used_bytes: memory_used,
            memory_peak_bytes: memory_peak,
            hit_rate,
            total_hits: keyspace_hits,
            total_misses: keyspace_misses,
        })
    }

    fn extract_redis_stat(info: &str, key: &str) -> u64 {
        info.lines()
            .find(|line| line.starts_with(key))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    }

    // Cache management
    pub async fn clear_cache(&self, pattern: Option<&str>) -> Result<u32> {
        let mut conn = self.get_connection().await?;

        let pattern = pattern.unwrap_or("*");
        let keys: Vec<String> = conn.keys(pattern)
            .map_err(|e| OmenError::CacheError(format!("Cache keys lookup failed: {}", e)))?;

        if keys.is_empty() {
            self.return_connection(conn).await;
            return Ok(0);
        }

        let deleted: u32 = conn.del(&keys)
            .map_err(|e| OmenError::CacheError(format!("Cache clear failed: {}", e)))?;

        self.return_connection(conn).await;

        info!("🧹 Cleared {} cache entries with pattern '{}'", deleted, pattern);
        Ok(deleted)
    }

    pub async fn warm_cache(&self) -> Result<()> {
        info!("🔥 Warming up Redis cache...");

        // Pre-load common provider health checks
        let common_providers = vec!["ollama", "openai", "anthropic", "google"];

        for provider_id in common_providers {
            self.cache_provider_health(provider_id, true, 100, None).await?;
        }

        info!("✅ Cache warming completed");
        Ok(())
    }
}

// Cache key generation utilities
impl RedisCache {
    pub fn generate_response_cache_key(
        &self,
        user_id: &str,
        messages: &[ChatMessage],
        model: &str,
        temperature: Option<f32>,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        model.hash(&mut hasher);
        temperature.unwrap_or(0.7).to_bits().hash(&mut hasher);

        // Hash message content
        for msg in messages {
            msg.role.hash(&mut hasher);
            match &msg.content {
                MessageContent::Text(text) => text.hash(&mut hasher),
                MessageContent::Parts(parts) => {
                    for part in parts {
                        match part {
                            ContentPart::Text { text } => text.hash(&mut hasher),
                            ContentPart::ImageUrl { image_url } => {
                                image_url.url.hash(&mut hasher);
                            }
                        }
                    }
                }
            }
        }

        format!("resp:{}:{:x}", user_id, hasher.finish())
    }

    pub fn generate_session_cache_key(&self, session_id: Uuid) -> String {
        format!("session:{}", session_id)
    }

    pub fn generate_rate_limit_key(&self, user_id: &str, window_type: &str) -> String {
        let now = chrono::Utc::now();
        let window_id = match window_type {
            "minute" => now.format("%Y%m%d%H%M").to_string(),
            "hour" => now.format("%Y%m%d%H").to_string(),
            "day" => now.format("%Y%m%d").to_string(),
            _ => now.format("%Y%m%d%H%M").to_string(),
        };
        format!("rate:{}:{}:{}", user_id, window_type, window_id)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub memory_used_bytes: u64,
    pub memory_peak_bytes: u64,
    pub hit_rate: f64,
    pub total_hits: u64,
    pub total_misses: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_key_generation() {
        let config = CacheConfig::default();
        let cache = RedisCache::new(config).unwrap();

        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }
        ];

        let key1 = cache.generate_response_cache_key("user1", &messages, "gpt-4", Some(0.7));
        let key2 = cache.generate_response_cache_key("user1", &messages, "gpt-4", Some(0.7));
        let key3 = cache.generate_response_cache_key("user2", &messages, "gpt-4", Some(0.7));

        assert_eq!(key1, key2); // Same user, same input = same key
        assert_ne!(key1, key3); // Different user = different key
    }

    #[tokio::test]
    async fn test_session_cache_key() {
        let config = CacheConfig::default();
        let cache = RedisCache::new(config).unwrap();

        let session_id = Uuid::new_v4();
        let key = cache.generate_session_cache_key(session_id);

        assert!(key.starts_with("session:"));
        assert!(key.contains(&session_id.to_string()));
    }
}