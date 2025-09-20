use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub routing: RoutingConfig,
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_db_url")]
    pub db: String,
    #[serde(default)]
    pub redis: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    #[serde(default)]
    pub prefer_local_for: Vec<String>,
    #[serde(default = "default_budget")]
    pub budget_monthly_usd: f64,
    #[serde(default)]
    pub soft_limits: HashMap<String, f64>,
    #[serde(default = "default_auto_swap")]
    pub auto_swap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub openai: ProviderConfig,
    #[serde(default)]
    pub anthropic: ProviderConfig,
    #[serde(default)]
    pub google: ProviderConfig,
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub azure: AzureConfig,
    #[serde(default)]
    pub xai: ProviderConfig,
    #[serde(default)]
    pub bedrock: BedrockConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OllamaConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoints: Vec<String>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AzureConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BedrockConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub access_key_id: Option<String>,
    #[serde(default)]
    pub secret_access_key: Option<String>,
    #[serde(default)]
    pub session_token: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    #[serde(default)]
    pub require_api_key: bool,
    #[serde(default)]
    pub master_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub enable_access_logs: bool,
}

// Default values
fn default_bind() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_workers() -> usize {
    num_cpus::get()
}

fn default_timeout() -> u64 {
    30
}

fn default_db_url() -> String {
    "sqlite:///data/omen.db".to_string()
}

fn default_budget() -> f64 {
    100.0
}

fn default_auto_swap() -> bool {
    false
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_redis_url")]
    pub redis_url: String,
    #[serde(default = "default_cache_ttl")]
    pub default_ttl_seconds: u64,
    #[serde(default = "default_response_cache_ttl")]
    pub response_cache_ttl: u64,
    #[serde(default = "default_session_cache_ttl")]
    pub session_cache_ttl: u64,
    #[serde(default = "default_rate_limit_ttl")]
    pub rate_limit_ttl: u64,
    #[serde(default = "default_provider_health_ttl")]
    pub provider_health_ttl: u64,
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size_mb: u64,
    #[serde(default)]
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: default_redis_url(),
            default_ttl_seconds: default_cache_ttl(),
            response_cache_ttl: default_response_cache_ttl(),
            session_cache_ttl: default_session_cache_ttl(),
            rate_limit_ttl: default_rate_limit_ttl(),
            provider_health_ttl: default_provider_health_ttl(),
            max_cache_size_mb: default_max_cache_size(),
            enabled: false, // Disabled by default
        }
    }
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

fn default_cache_ttl() -> u64 {
    3600 // 1 hour
}

fn default_response_cache_ttl() -> u64 {
    1800 // 30 minutes
}

fn default_session_cache_ttl() -> u64 {
    7200 // 2 hours
}

fn default_rate_limit_ttl() -> u64 {
    60 // 1 minute
}

fn default_provider_health_ttl() -> u64 {
    300 // 5 minutes
}

fn default_max_cache_size() -> u64 {
    1024 // 1GB
}

impl Config {
    pub async fn load(path: &str) -> Result<Self> {
        use std::fs;

        // Try to load from file first
        if let Ok(content) = fs::read_to_string(path) {
            let mut config: Config = toml::from_str(&content)?;
            config.apply_env_overrides();
            return Ok(config);
        }

        // If no config file, create default and apply env overrides
        let mut config = Self::default();
        config.apply_env_overrides();
        Ok(config)
    }

    fn apply_env_overrides(&mut self) {
        // Server config
        if let Ok(bind) = std::env::var("OMEN_BIND") {
            self.server.bind = bind;
        }
        if let Ok(port) = std::env::var("OMEN_PORT") {
            if let Ok(port_num) = port.parse() {
                self.server.port = port_num;
            }
        }

        // Storage config
        if let Ok(db_url) = std::env::var("OMEN_DB_URL") {
            self.storage.db = db_url;
        }
        if let Ok(redis_url) = std::env::var("OMEN_REDIS_URL") {
            self.storage.redis = Some(redis_url);
        }

        // Provider configs
        if let Ok(api_key) = std::env::var("OMEN_OPENAI_API_KEY") {
            self.providers.openai.enabled = true;
            self.providers.openai.api_key = Some(api_key);
        }
        if let Ok(api_key) = std::env::var("OMEN_ANTHROPIC_API_KEY") {
            self.providers.anthropic.enabled = true;
            self.providers.anthropic.api_key = Some(api_key);
        }
        if let Ok(api_key) = std::env::var("OMEN_GOOGLE_API_KEY") {
            self.providers.google.enabled = true;
            self.providers.google.api_key = Some(api_key);
        }
        if let Ok(api_key) = std::env::var("OMEN_XAI_API_KEY") {
            self.providers.xai.enabled = true;
            self.providers.xai.api_key = Some(api_key);
        }

        // Azure config
        if let Ok(endpoint) = std::env::var("OMEN_AZURE_OPENAI_ENDPOINT") {
            self.providers.azure.enabled = true;
            self.providers.azure.endpoint = Some(endpoint);
            if let Ok(api_key) = std::env::var("OMEN_AZURE_OPENAI_API_KEY") {
                self.providers.azure.api_key = Some(api_key);
            }
        }

        // Ollama config
        if let Ok(endpoints) = std::env::var("OMEN_OLLAMA_ENDPOINTS") {
            self.providers.ollama.enabled = true;
            self.providers.ollama.endpoints = endpoints
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        // Bedrock config
        if let Ok(region) = std::env::var("AWS_REGION") {
            if let Ok(access_key) = std::env::var("AWS_ACCESS_KEY_ID") {
                if let Ok(secret_key) = std::env::var("AWS_SECRET_ACCESS_KEY") {
                    self.providers.bedrock.enabled = true;
                    self.providers.bedrock.region = Some(region);
                    self.providers.bedrock.access_key_id = Some(access_key);
                    self.providers.bedrock.secret_access_key = Some(secret_key);

                    if let Ok(session_token) = std::env::var("AWS_SESSION_TOKEN") {
                        self.providers.bedrock.session_token = Some(session_token);
                    }
                }
            }
        }

        // Routing config
        if let Ok(prefer) = std::env::var("OMEN_ROUTER_PREFER_LOCAL_FOR") {
            self.routing.prefer_local_for = prefer
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }
        if let Ok(budget) = std::env::var("OMEN_BUDGET_MONTHLY_USD") {
            if let Ok(budget_num) = budget.parse() {
                self.routing.budget_monthly_usd = budget_num;
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind: default_bind(),
                port: default_port(),
                workers: default_workers(),
                timeout_seconds: default_timeout(),
            },
            storage: StorageConfig {
                db: default_db_url(),
                redis: None,
            },
            routing: RoutingConfig {
                prefer_local_for: vec!["code".to_string(), "regex".to_string(), "tests".to_string()],
                budget_monthly_usd: default_budget(),
                soft_limits: HashMap::new(),
                auto_swap: default_auto_swap(),
            },
            providers: ProvidersConfig {
                openai: ProviderConfig::default(),
                anthropic: ProviderConfig::default(),
                google: ProviderConfig::default(),
                ollama: OllamaConfig::default(),
                azure: AzureConfig::default(),
                xai: ProviderConfig::default(),
                bedrock: BedrockConfig::default(),
            },
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}