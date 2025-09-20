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
use tracing::{debug, error};

#[derive(Debug)]
pub struct OllamaProvider {
    client: Client,
    endpoints: Vec<String>,
}

impl OllamaProvider {
    pub async fn new(endpoints: Vec<String>, timeout_seconds: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        debug!("✅ Ollama provider initialized with {} endpoints", endpoints.len());

        let provider = Self { client, endpoints };

        Ok(provider)
    }

    async fn get_healthy_endpoint(&self) -> Option<String> {
        for endpoint in &self.endpoints {
            if self.check_endpoint_health(endpoint).await {
                return Some(endpoint.clone());
            }
        }
        None
    }

    async fn check_endpoint_health(&self, endpoint: &str) -> bool {
        let response = self
            .client
            .get(&format!("{}/api/tags", endpoint))
            .send()
            .await;

        match response {
            Ok(resp) => resp.status().is_success(),
            Err(e) => {
                debug!("Ollama endpoint {} health check failed: {}", endpoint, e);
                false
            }
        }
    }

    fn openai_to_ollama_messages(&self, messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|msg| {
                json!({
                    "role": msg.role,
                    "content": msg.content.to_string(),
                })
            })
            .collect()
    }

    fn ollama_to_openai_response(
        &self,
        ollama_response: serde_json::Value,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let content = ollama_response["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Ollama doesn't provide token counts, so we estimate
        let estimated_prompt_tokens = 10; // Rough estimate
        let estimated_completion_tokens = content.split_whitespace().count() as u32;

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
                prompt_tokens: estimated_prompt_tokens,
                completion_tokens: estimated_completion_tokens,
                total_tokens: estimated_prompt_tokens + estimated_completion_tokens,
            },
            system_fingerprint: None,
        })
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn id(&self) -> &str {
        "ollama"
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Ollama
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.get_healthy_endpoint().await.is_some())
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        let endpoint = self
            .get_healthy_endpoint()
            .await
            .ok_or_else(|| OmenError::ProviderUnavailable("No healthy Ollama endpoints".to_string()))?;

        let response = self
            .client
            .get(&format!("{}/api/tags", endpoint))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(OmenError::Provider(format!(
                "Ollama API error: {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response.json().await?;
        let mut models = Vec::new();

        if let Some(model_list) = data["models"].as_array() {
            for model_data in model_list {
                if let Some(name) = model_data["name"].as_str() {
                    // Extract model info
                    let _size = model_data["size"].as_u64().unwrap_or(0);
                    let context_length = estimate_context_length(name);

                    models.push(Model {
                        id: name.to_string(),
                        object: "model".to_string(),
                        created: chrono::Utc::now().timestamp(),
                        owned_by: "ollama".to_string(),
                        provider: "ollama".to_string(),
                        context_length,
                        pricing: ModelPricing {
                            input_per_1k: 0.0,  // Local models are free
                            output_per_1k: 0.0,
                        },
                        capabilities: ModelCapabilities {
                            vision: name.contains("vision") || name.contains("llava"),
                            functions: false, // Most Ollama models don't support function calling
                            streaming: true,
                        },
                    });
                }
            }
        }

        Ok(models)
    }

    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<ChatCompletionResponse> {
        let endpoint = self
            .get_healthy_endpoint()
            .await
            .ok_or_else(|| OmenError::ProviderUnavailable("No healthy Ollama endpoints".to_string()))?;

        let messages = self.openai_to_ollama_messages(&request.messages);

        let mut payload = json!({
            "model": request.model,
            "messages": messages,
            "stream": false,
        });

        // Add optional parameters
        if let Some(temp) = request.temperature {
            payload["options"] = json!({
                "temperature": temp,
            });
        }

        debug!("Sending request to Ollama: {}", context.request_id);

        let response = self
            .client
            .post(&format!("{}/api/chat", endpoint))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Ollama API error: {}", error_text);
            return Err(OmenError::Provider(format!(
                "Ollama API error: {}",
                error_text
            )));
        }

        let ollama_response: serde_json::Value = response.json().await?;
        self.ollama_to_openai_response(
            ollama_response,
            &context.request_id.to_string(),
            &request.model,
        )
    }

    async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        let endpoint = self
            .get_healthy_endpoint()
            .await
            .ok_or_else(|| OmenError::ProviderUnavailable("No healthy Ollama endpoints".to_string()))?;

        let messages = self.openai_to_ollama_messages(&request.messages);

        let mut payload = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(temp) = request.temperature {
            payload["options"] = json!({
                "temperature": temp,
            });
        }

        debug!("Sending streaming request to Ollama: {}", context.request_id);

        let response = self
            .client
            .post(&format!("{}/api/chat", endpoint))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OmenError::Provider(format!(
                "Ollama API error: {}",
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
                        // Ollama sends JSONL format, convert to OpenAI SSE
                        if let Ok(ollama_chunk) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(message) = ollama_chunk.get("message") {
                                if let Some(content) = message.get("content") {
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
                                            finish_reason: if ollama_chunk.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                                                Some("stop".to_string())
                                            } else {
                                                None
                                            },
                                        }],
                                        system_fingerprint: None,
                                    };
                                    return Ok(format!("data: {}\n\n", serde_json::to_string(&openai_chunk).unwrap_or_default()));
                                }
                            }

                            // Check if this is the final chunk
                            if ollama_chunk.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                                return Ok("data: [DONE]\n\n".to_string());
                            }
                        }
                        Ok(String::new()) // Skip malformed chunks
                    }
                    Err(e) => Err(OmenError::HttpClient(e)),
                }
            })
            .filter(|result| {
                // Filter out empty strings
                futures::future::ready(if let Ok(s) = result {
                    !s.is_empty()
                } else {
                    true
                })
            });

        Ok(Box::new(stream))
    }
}

fn estimate_context_length(model_name: &str) -> u32 {
    if model_name.contains("7b") {
        4096
    } else if model_name.contains("13b") {
        4096
    } else if model_name.contains("70b") {
        4096
    } else if model_name.contains("llama3") {
        8192
    } else if model_name.contains("qwen") {
        8192
    } else if model_name.contains("deepseek") {
        16384
    } else {
        4096 // Default
    }
}