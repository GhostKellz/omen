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
#[allow(dead_code)]
pub struct VertexAIProvider {
    client: Client,
    project_id: String,
    location: String,
    access_token: Option<String>,
    base_url: String,
}

impl VertexAIProvider {
    pub async fn new(
        project_id: String,
        location: Option<String>,
        access_token: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        let location = location.unwrap_or_else(|| "us-central1".to_string());
        let base_url = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}",
            location, project_id, location
        );

        let provider = Self {
            client,
            project_id,
            location,
            access_token,
            base_url,
        };

        debug!("✅ Vertex AI provider initialized for project {}", provider.project_id);

        Ok(provider)
    }

    async fn get_access_token(&self) -> Result<String> {
        // If token provided via env var, use it
        if let Some(token) = &self.access_token {
            return Ok(token.clone());
        }

        // Otherwise, try to get token from gcloud CLI
        let output = tokio::process::Command::new("gcloud")
            .args(["auth", "application-default", "print-access-token"])
            .output()
            .await
            .map_err(|e| OmenError::Provider(format!("Failed to get Google Cloud access token: {}. Please run 'gcloud auth application-default login'", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OmenError::Provider(format!(
                "Failed to get Google Cloud access token: {}. Please run 'gcloud auth application-default login'",
                stderr
            )));
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(token)
    }

    fn openai_to_vertex_messages(&self, messages: &[ChatMessage]) -> (String, Vec<serde_json::Value>) {
        let mut system_prompt = String::new();
        let mut vertex_messages = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    if !system_prompt.is_empty() {
                        system_prompt.push('\n');
                    }
                    system_prompt.push_str(&msg.content.text());
                }
                "user" | "assistant" => {
                    vertex_messages.push(json!({
                        "role": msg.role,
                        "content": msg.content
                    }));
                }
                _ => {
                    warn!("Skipping unsupported role: {}", msg.role);
                }
            }
        }

        (system_prompt, vertex_messages)
    }

    fn vertex_to_openai_response(
        &self,
        vertex_response: serde_json::Value,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let content = vertex_response["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage_data = &vertex_response["usage"];
        let input_tokens = usage_data["inputTokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = usage_data["outputTokens"].as_u64().unwrap_or(0) as u32;

        Ok(ChatCompletionResponse {
            id: request_id.to_string(),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: crate::types::MessageContent::Text(content),
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
impl Provider for VertexAIProvider {
    fn id(&self) -> &str {
        "vertexai"
    }

    fn name(&self) -> &str {
        "Google Vertex AI (Claude)"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::VertexAI
    }

    async fn health_check(&self) -> Result<bool> {
        // Try to get access token as health check
        match self.get_access_token().await {
            Ok(_) => {
                debug!("✅ Vertex AI health check passed");
                Ok(true)
            }
            Err(e) => {
                warn!("Vertex AI health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        // Return Claude models available on Vertex AI (2025)
        Ok(vec![
            Model {
                id: "claude-sonnet-4-5@20250929".to_string(),
                object: "model".to_string(),
                created: 1748649600, // September 2025
                owned_by: "anthropic".to_string(),
                provider: "vertexai".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    input_per_1k: 0.003,
                    output_per_1k: 0.015,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: true,
                    streaming: true,
                },
            },
            Model {
                id: "claude-opus-4-1@20250805".to_string(),
                object: "model".to_string(),
                created: 1754438400, // August 2025
                owned_by: "anthropic".to_string(),
                provider: "vertexai".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    input_per_1k: 0.015,
                    output_per_1k: 0.075,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: true,
                    streaming: true,
                },
            },
            Model {
                id: "claude-sonnet-4@20250514".to_string(),
                object: "model".to_string(),
                created: 1747180800, // May 2025
                owned_by: "anthropic".to_string(),
                provider: "vertexai".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    input_per_1k: 0.003,
                    output_per_1k: 0.015,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: true,
                    streaming: true,
                },
            },
            Model {
                id: "claude-3-7-sonnet@20250219".to_string(),
                object: "model".to_string(),
                created: 1739923200, // February 2025
                owned_by: "anthropic".to_string(),
                provider: "vertexai".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    input_per_1k: 0.003,
                    output_per_1k: 0.015,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: true,
                    streaming: true,
                },
            },
            Model {
                id: "claude-3-5-sonnet@20241022".to_string(),
                object: "model".to_string(),
                created: 1729555200, // Oct 2024
                owned_by: "anthropic".to_string(),
                provider: "vertexai".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    input_per_1k: 0.003,
                    output_per_1k: 0.015,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: true,
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
        let token = self.get_access_token().await?;
        let (system_prompt, messages) = self.openai_to_vertex_messages(&request.messages);

        let mut payload = json!({
            "anthropicVersion": "vertex-2023-10-16",
            "messages": messages,
            "maxTokens": request.max_tokens.unwrap_or(1000),
        });

        if !system_prompt.is_empty() {
            payload["systemInstruction"] = json!(system_prompt);
        }

        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            payload["topP"] = json!(top_p);
        }

        debug!("Sending request to Vertex AI: {}", context.request_id);

        let response = self
            .client
            .post(&format!(
                "{}/publishers/anthropic/models/{}:rawPredict",
                self.base_url, request.model
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Vertex AI API error: {}", error_text);
            return Err(OmenError::Provider(format!(
                "Vertex AI API error: {}",
                error_text
            )));
        }

        let vertex_response: serde_json::Value = response.json().await?;
        self.vertex_to_openai_response(
            vertex_response,
            &context.request_id.to_string(),
            &request.model,
        )
    }

    async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        let token = self.get_access_token().await?;
        let (system_prompt, messages) = self.openai_to_vertex_messages(&request.messages);

        let mut payload = json!({
            "anthropicVersion": "vertex-2023-10-16",
            "messages": messages,
            "maxTokens": request.max_tokens.unwrap_or(1000),
            "stream": true,
        });

        if !system_prompt.is_empty() {
            payload["systemInstruction"] = json!(system_prompt);
        }

        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }

        debug!("Sending streaming request to Vertex AI: {}", context.request_id);

        let response = self
            .client
            .post(&format!(
                "{}/publishers/anthropic/models/{}:streamRawPredict",
                self.base_url, request.model
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OmenError::Provider(format!(
                "Vertex AI API error: {}",
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
                        // Convert Vertex AI SSE to OpenAI format
                        if text.starts_with("data: ") {
                            let json_str = text.trim_start_matches("data: ").trim();
                            if let Ok(vertex_chunk) = serde_json::from_str::<serde_json::Value>(json_str) {
                                // Convert to OpenAI chunk format
                                if let Some(delta) = vertex_chunk.get("delta") {
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
