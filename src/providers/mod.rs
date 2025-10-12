use crate::{
    config::Config,
    error::Result,
    types::*,
};
use async_trait::async_trait;
use futures::stream::Stream;
use std::{collections::HashMap, sync::Arc};

pub mod anthropic;
pub mod azure;
pub mod bedrock;
pub mod google;
pub mod ollama;
pub mod openai;
pub mod vertexai;
pub mod xai;

pub use anthropic::AnthropicProvider;
pub use azure::AzureProvider;
pub use bedrock::BedrockProvider;
pub use google::GoogleProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use vertexai::VertexAIProvider;
pub use xai::XaiProvider;

#[async_trait]
pub trait Provider: Send + Sync + std::fmt::Debug {
    /// Unique identifier for this provider
    fn id(&self) -> &str;

    /// Human-readable name for this provider
    fn name(&self) -> &str;

    /// Type of provider
    fn provider_type(&self) -> ProviderType;

    /// Check if the provider is healthy and available
    async fn health_check(&self) -> Result<bool>;

    /// List available models from this provider
    async fn list_models(&self) -> Result<Vec<Model>>;

    /// Perform a chat completion
    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<ChatCompletionResponse>;

    /// Stream a chat completion
    async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>>;
}

#[derive(Debug)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    pub async fn new(config: &Config) -> Result<Self> {
        let mut providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();

        // Initialize OpenAI provider
        if config.providers.openai.enabled {
            if let Some(ref api_key) = config.providers.openai.api_key {
                let provider = Arc::new(
                    OpenAIProvider::new(
                        api_key.clone(),
                        config.providers.openai.base_url.clone(),
                        config.providers.openai.timeout_seconds,
                    ).await?
                );
                providers.insert("openai".to_string(), provider);
            }
        }

        // Initialize Anthropic provider
        if config.providers.anthropic.enabled {
            if let Some(ref api_key) = config.providers.anthropic.api_key {
                let provider = Arc::new(
                    AnthropicProvider::new(
                        api_key.clone(),
                        config.providers.anthropic.base_url.clone(),
                        config.providers.anthropic.timeout_seconds,
                    ).await?
                );
                providers.insert("anthropic".to_string(), provider);
            }
        }

        // Initialize Google provider
        if config.providers.google.enabled {
            if let Some(ref api_key) = config.providers.google.api_key {
                let provider = Arc::new(
                    GoogleProvider::new(
                        api_key.clone(),
                        config.providers.google.base_url.clone(),
                        config.providers.google.timeout_seconds,
                    ).await?
                );
                providers.insert("google".to_string(), provider);
            }
        }

        // Initialize Azure provider
        if config.providers.azure.enabled {
            if let Some(ref endpoint) = config.providers.azure.endpoint {
                if let Some(ref api_key) = config.providers.azure.api_key {
                    let provider = Arc::new(
                        AzureProvider::new(
                            endpoint.clone(),
                            api_key.clone(),
                            config.providers.azure.api_version.clone(),
                            config.providers.azure.timeout_seconds,
                        ).await?
                    );
                    providers.insert("azure".to_string(), provider);
                }
            }
        }

        // Initialize XAI provider
        if config.providers.xai.enabled {
            if let Some(ref api_key) = config.providers.xai.api_key {
                let provider = Arc::new(
                    XaiProvider::new(
                        api_key.clone(),
                        config.providers.xai.base_url.clone(),
                        config.providers.xai.timeout_seconds,
                    ).await?
                );
                providers.insert("xai".to_string(), provider);
            }
        }

        // Initialize Ollama provider
        if config.providers.ollama.enabled && !config.providers.ollama.endpoints.is_empty() {
            let provider = Arc::new(
                OllamaProvider::new(
                    config.providers.ollama.endpoints.clone(),
                    config.providers.ollama.timeout_seconds,
                ).await?
            );
            providers.insert("ollama".to_string(), provider);
        }

        // Initialize Bedrock provider
        if config.providers.bedrock.enabled {
            if let (Some(region), Some(access_key), Some(secret_key)) = (
                &config.providers.bedrock.region,
                &config.providers.bedrock.access_key_id,
                &config.providers.bedrock.secret_access_key,
            ) {
                let provider = Arc::new(
                    BedrockProvider::new(
                        region.clone(),
                        access_key.clone(),
                        secret_key.clone(),
                        config.providers.bedrock.session_token.clone(),
                        config.providers.bedrock.timeout_seconds,
                    ).await?
                );
                providers.insert("bedrock".to_string(), provider);
            }
        }

        // Initialize Vertex AI provider (Claude via Google Cloud)
        if config.providers.vertexai.enabled {
            if let Some(ref project_id) = config.providers.vertexai.project_id {
                let provider = Arc::new(
                    VertexAIProvider::new(
                        project_id.clone(),
                        config.providers.vertexai.location.clone(),
                        config.providers.vertexai.access_token.clone(),
                        config.providers.vertexai.timeout_seconds,
                    ).await?
                );
                providers.insert("vertexai".to_string(), provider);
            }
        }

        Ok(Self { providers })
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(id).cloned()
    }

    pub fn all(&self) -> Vec<Arc<dyn Provider>> {
        self.providers.values().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }
}