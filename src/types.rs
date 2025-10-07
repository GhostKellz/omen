use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Embeddings request/response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsRequest {
    pub input: EmbeddingInput,
    pub model: String,
    #[serde(default)]
    pub encoding_format: Option<String>,
    #[serde(default)]
    pub dimensions: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    pub object: String,
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: EmbeddingUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

// Text completions (legacy endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub prompt: CompletionPrompt,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub n: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CompletionPrompt {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<CompletionChoice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionChoice {
    pub text: String,
    pub index: u32,
    pub finish_reason: Option<String>,
}

// OpenAI-compatible chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
    #[serde(default)]
    pub presence_penalty: Option<f32>,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
    #[serde(default)]
    pub tools: Option<Vec<Tool>>,
    #[serde(default)]
    pub tool_choice: Option<ToolChoice>,
    // OMEN-specific extensions
    #[serde(default)]
    pub tags: Option<HashMap<String, String>>,
    #[serde(default)]
    pub omen: Option<OmenConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: MessageContent,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    // Simple text string (most common)
    Text(String),
    // Array of content parts (OpenAI multimodal format)
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    ImageUrl {
        image_url: ImageUrl,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(default)]
    pub detail: Option<String>,
}

impl MessageContent {
    pub fn text(&self) -> String {
        match self {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Parts(parts) => {
                parts.iter()
                    .filter_map(|part| match part {
                        ContentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    }

    pub fn has_images(&self) -> bool {
        match self {
            MessageContent::Text(_) => false,
            MessageContent::Parts(parts) => {
                parts.iter().any(|part| matches!(part, ContentPart::ImageUrl { .. }))
            }
        }
    }

    pub fn len(&self) -> usize {
        self.text().len()
    }

    pub fn is_multimodal(&self) -> bool {
        self.has_images()
    }

    pub fn to_lowercase(&self) -> String {
        self.text().to_lowercase()
    }
}

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageContent::Text(text) => write!(f, "{}", text),
            MessageContent::Parts(parts) => {
                let text_parts: Vec<&str> = parts.iter()
                    .filter_map(|part| match part {
                        ContentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                write!(f, "{}", text_parts.join(" "))?;

                let image_count = parts.iter()
                    .filter(|part| matches!(part, ContentPart::ImageUrl { .. }))
                    .count();
                if image_count > 0 {
                    write!(f, " [+{} image(s)]", image_count)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    #[serde(rename = "type")]
    pub image_type: ImageType,
    pub data: ImageData,
    #[serde(default)]
    pub detail: Option<ImageDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageType {
    #[serde(rename = "image_url")]
    ImageUrl,
    #[serde(rename = "image_base64")]
    ImageBase64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageData {
    Url { url: String },
    Base64 {
        media_type: String, // e.g., "image/jpeg", "image/png"
        data: String,       // base64 encoded image
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    Low,
    High,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto,
    None,
    Function { function: ToolFunctionChoice },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunctionChoice {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Usage,
    #[serde(default)]
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// Streaming response chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoiceDelta>,
    #[serde(default)]
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoiceDelta {
    pub index: u32,
    pub delta: ChatMessageDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDelta {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
    // OMEN extensions
    pub provider: String,
    pub context_length: u32,
    pub pricing: ModelPricing,
    pub capabilities: ModelCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_per_1k: f64,
    pub output_per_1k: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub vision: bool,
    pub functions: bool,
    pub streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<Model>,
}

// Health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub service: String,
    pub timestamp: String,
    pub providers: Vec<ProviderHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub id: String,
    pub name: String,
    pub healthy: bool,
    pub models_count: usize,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub avg_cost_per_1k: Option<f64>,
    #[serde(default)]
    pub success_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderScore {
    pub provider_id: String,
    pub provider_name: String,
    pub health_score: f64,
    pub latency_ms: u64,
    pub cost_score: f64,
    pub reliability_score: f64,
    pub overall_score: f64,
    pub recommended: bool,
}

// OMEN-specific configuration for advanced routing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmenConfig {
    /// Routing strategy: single, race, speculate_k, parallel_merge
    #[serde(default)]
    pub strategy: Option<String>,
    /// Number of providers for speculate_k/parallel_merge strategies
    #[serde(default)]
    pub k: Option<u32>,
    /// Optional allowlist of providers
    #[serde(default)]
    pub providers: Option<Vec<String>>,
    /// Budget cap per request in USD
    #[serde(default)]
    pub budget_usd: Option<f64>,
    /// Maximum latency in milliseconds before canceling slower providers
    #[serde(default)]
    pub max_latency_ms: Option<u32>,
    /// Session stickiness: none, turn, session
    #[serde(default)]
    pub stickiness: Option<String>,
    /// Custom priority weights for providers
    #[serde(default)]
    pub priority_weights: Option<HashMap<String, f32>>,
    /// Minimum useful token threshold for race conditions
    #[serde(default)]
    pub min_useful_tokens: Option<u32>,
}

impl Default for OmenConfig {
    fn default() -> Self {
        Self {
            strategy: Some("single".to_string()),
            k: Some(2),
            providers: None,
            budget_usd: Some(0.10),
            max_latency_ms: Some(5000),
            stickiness: Some("turn".to_string()),
            priority_weights: None,
            min_useful_tokens: Some(5),
        }
    }
}

// Internal types
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: Uuid,
    pub user_id: Option<String>,
    pub api_key: Option<String>,
    pub intent: Option<String>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Google,
    Ollama,
    Azure,
    Xai,
    Bedrock,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Google => write!(f, "google"),
            ProviderType::Ollama => write!(f, "ollama"),
            ProviderType::Azure => write!(f, "azure"),
            ProviderType::Xai => write!(f, "xai"),
            ProviderType::Bedrock => write!(f, "bedrock"),
        }
    }
}