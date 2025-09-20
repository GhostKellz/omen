use crate::{
    error::{OmenError, Result},
    providers::Provider,
    types::*,
};
use async_trait::async_trait;
use futures::{stream::Stream, StreamExt};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{debug, error, warn};

#[derive(Debug)]
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub async fn new(
        api_key: String,
        base_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        let base_url = base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string());

        let provider = Self {
            client,
            api_key,
            base_url,
        };

        debug!("✅ Anthropic provider initialized");

        Ok(provider)
    }

    fn openai_to_anthropic_messages(&self, messages: &[ChatMessage]) -> (String, Vec<serde_json::Value>) {
        let mut system_message = String::new();
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    if !system_message.is_empty() {
                        system_message.push('\n');
                    }
                    system_message.push_str(&msg.content);
                }
                "user" | "assistant" => {
                    anthropic_messages.push(json!({
                        "role": msg.role,
                        "content": msg.content
                    }));
                }
                _ => {
                    // Skip unsupported roles
                    warn!("Skipping unsupported role: {}", msg.role);
                }
            }
        }

        (system_message, anthropic_messages)
    }

    fn anthropic_to_openai_response(
        &self,
        anthropic_response: serde_json::Value,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let content = anthropic_response["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage_data = &anthropic_response["usage"];
        let input_tokens = usage_data["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = usage_data["output_tokens"].as_u64().unwrap_or(0) as u32;

        Ok(ChatCompletionResponse {
            id: request_id.to_string(),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content,
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Usage {
                prompt_tokens: input_tokens,
                completion_tokens: output_tokens,
                total_tokens: input_tokens + output_tokens,
            },
            system_fingerprint: None,
        })
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn id(&self) -> &str {
        "anthropic"
    }

    fn name(&self) -> &str {
        "Anthropic"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    async fn health_check(&self) -> Result<bool> {
        // Anthropic doesn't have a dedicated health endpoint, so we'll just check if we can reach the API
        let response = self
            .client
            .post(&format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": "claude-3-haiku-20240307",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "test"}]
            }))
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().as_u16() != 500), // 400 is OK for health check
            Err(e) => {
                debug!("Anthropic health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        // Anthropic doesn't have a models endpoint, so we'll return the known models
        Ok(vec![
            Model {
                id: "claude-3-opus-20240229".to_string(),
                object: "model".to_string(),
                created: 1709251200,
                owned_by: "anthropic".to_string(),
                provider: "anthropic".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    input_per_1k: 0.015,
                    output_per_1k: 0.075,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: false,
                    streaming: true,
                },
            },
            Model {
                id: "claude-3-sonnet-20240229".to_string(),
                object: "model".to_string(),
                created: 1709251200,
                owned_by: "anthropic".to_string(),
                provider: "anthropic".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    input_per_1k: 0.003,
                    output_per_1k: 0.015,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: false,
                    streaming: true,
                },
            },
            Model {
                id: "claude-3-haiku-20240307".to_string(),
                object: "model".to_string(),
                created: 1709856000,
                owned_by: "anthropic".to_string(),
                provider: "anthropic".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    input_per_1k: 0.00025,
                    output_per_1k: 0.00125,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: false,
                    streaming: true,
                },
            },
        ])
    }

    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<ChatCompletionResponse> {
        let (system_message, messages) = self.openai_to_anthropic_messages(&request.messages);

        let mut payload = json!({
            "model": request.model,
            "max_tokens": request.max_tokens.unwrap_or(1000),
            "messages": messages,
        });

        if !system_message.is_empty() {
            payload["system"] = json!(system_message);
        }

        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            payload["top_p"] = json!(top_p);
        }

        debug!("Sending request to Anthropic: {}", context.request_id);

        let response = self
            .client
            .post(&format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Anthropic API error: {}", error_text);
            return Err(OmenError::Provider(format!(
                "Anthropic API error: {}",
                error_text
            )));
        }

        let anthropic_response: serde_json::Value = response.json().await?;
        self.anthropic_to_openai_response(
            anthropic_response,
            &context.request_id.to_string(),
            &request.model,
        )
    }

    async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        let (system_message, messages) = self.openai_to_anthropic_messages(&request.messages);

        let mut payload = json!({
            "model": request.model,
            "max_tokens": request.max_tokens.unwrap_or(1000),
            "messages": messages,
            "stream": true,
        });

        if !system_message.is_empty() {
            payload["system"] = json!(system_message);
        }

        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }

        debug!("Sending streaming request to Anthropic: {}", context.request_id);

        let response = self
            .client
            .post(&format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OmenError::Provider(format!(
                "Anthropic API error: {}",
                error_text
            )));
        }

        let request_id = context.request_id.to_string();
        let model = request.model.clone();

        let stream = response
            .bytes_stream()
            .map(move |chunk| {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        // Convert Anthropic SSE to OpenAI format
                        if text.starts_with("data: ") {
                            let json_str = text.trim_start_matches("data: ").trim();
                            if let Ok(anthropic_chunk) = serde_json::from_str::<serde_json::Value>(json_str) {
                                // Convert to OpenAI chunk format
                                if let Some(delta) = anthropic_chunk.get("delta") {
                                    if let Some(content) = delta.get("text") {
                                        let openai_chunk = ChatCompletionChunk {
                                            id: request_id.clone(),
                                            object: "chat.completion.chunk".to_string(),
                                            created: chrono::Utc::now().timestamp(),
                                            model: model.clone(),
                                            choices: vec![ChatChoiceDelta {
                                                index: 0,
                                                delta: ChatMessageDelta {
                                                    role: None,
                                                    content: content.as_str().map(|s| s.to_string()),
                                                    tool_calls: None,
                                                },
                                                finish_reason: None,
                                            }],
                                            system_fingerprint: None,
                                        };
                                        return Ok(format!("data: {}\n\n", serde_json::to_string(&openai_chunk).unwrap_or_default()));
                                    }
                                }
                            }
                        }
                        Ok(text.to_string())
                    }
                    Err(e) => Err(OmenError::HttpClient(e)),
                }
            });

        Ok(Box::new(stream))
    }
}