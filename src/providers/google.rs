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
pub struct GoogleProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GoogleProvider {
    pub async fn new(
        api_key: String,
        base_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        let base_url = base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());

        let provider = Self {
            client,
            api_key,
            base_url,
        };

        debug!("✅ Google Gemini provider initialized");

        Ok(provider)
    }

    fn openai_to_gemini_messages(&self, messages: &[ChatMessage]) -> (String, Vec<serde_json::Value>) {
        let mut system_instruction = String::new();
        let mut gemini_contents = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    if !system_instruction.is_empty() {
                        system_instruction.push('\n');
                    }
                    system_instruction.push_str(&msg.content.text());
                }
                "user" => {
                    gemini_contents.push(json!({
                        "role": "user",
                        "parts": [{"text": msg.content}]
                    }));
                }
                "assistant" => {
                    gemini_contents.push(json!({
                        "role": "model",
                        "parts": [{"text": msg.content}]
                    }));
                }
                _ => {
                    warn!("Skipping unsupported role: {}", msg.role);
                }
            }
        }

        (system_instruction, gemini_contents)
    }

    fn gemini_to_openai_response(
        &self,
        gemini_response: serde_json::Value,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let candidates = gemini_response["candidates"].as_array()
            .ok_or_else(|| OmenError::Provider("No candidates in Gemini response".to_string()))?;

        let first_candidate = candidates.first()
            .ok_or_else(|| OmenError::Provider("No candidates in Gemini response".to_string()))?;

        let parts = first_candidate["content"]["parts"].as_array()
            .ok_or_else(|| OmenError::Provider("No parts in Gemini candidate".to_string()))?;

        let content = parts.first()
            .and_then(|part| part["text"].as_str())
            .unwrap_or("")
            .to_string();

        // Gemini doesn't provide token counts in the same format
        let usage_metadata = gemini_response.get("usageMetadata");
        let prompt_tokens = usage_metadata
            .and_then(|u| u["promptTokenCount"].as_u64())
            .unwrap_or(0) as u32;
        let completion_tokens = usage_metadata
            .and_then(|u| u["candidatesTokenCount"].as_u64())
            .unwrap_or(0) as u32;

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
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            system_fingerprint: None,
        })
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    fn id(&self) -> &str {
        "google"
    }

    fn name(&self) -> &str {
        "Google Gemini"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Google
    }

    async fn health_check(&self) -> Result<bool> {
        // Test with a simple request to check API access
        let response = self
            .client
            .post(&format!("{}/v1beta/models/gemini-1.5-flash:generateContent", self.base_url))
            .header("Content-Type", "application/json")
            .query(&[("key", &self.api_key)])
            .json(&json!({
                "contents": [{
                    "parts": [{"text": "test"}]
                }]
            }))
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().as_u16() != 500), // 400 is OK for health check
            Err(e) => {
                debug!("Google Gemini health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        // Return current Gemini models (as of 2025)
        // Google has Gemini 2.5 and 2.0 models now
        Ok(vec![
            Model {
                id: "gemini-2.5-pro".to_string(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "google".to_string(),
                provider: "google".to_string(),
                context_length: 2097152, // 2M tokens
                pricing: ModelPricing {
                    input_per_1k: 0.00125,
                    output_per_1k: 0.005,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: true,
                    streaming: true,
                },
            },
            Model {
                id: "gemini-2.5-flash".to_string(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "google".to_string(),
                provider: "google".to_string(),
                context_length: 1048576, // 1M tokens
                pricing: ModelPricing {
                    input_per_1k: 0.000075,
                    output_per_1k: 0.0003,
                },
                capabilities: ModelCapabilities {
                    vision: true,
                    functions: true,
                    streaming: true,
                },
            },
            Model {
                id: "gemini-2.0-flash".to_string(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "google".to_string(),
                provider: "google".to_string(),
                context_length: 1048576, // 1M tokens
                pricing: ModelPricing {
                    input_per_1k: 0.0000375,
                    output_per_1k: 0.00015,
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
        let (system_instruction, contents) = self.openai_to_gemini_messages(&request.messages);

        let mut payload = json!({
            "contents": contents,
        });

        if !system_instruction.is_empty() {
            payload["systemInstruction"] = json!({
                "parts": [{"text": system_instruction}]
            });
        }

        // Add generation config
        let mut generation_config = json!({});
        if let Some(temp) = request.temperature {
            generation_config["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }
        if let Some(top_p) = request.top_p {
            generation_config["topP"] = json!(top_p);
        }
        if !generation_config.as_object().unwrap().is_empty() {
            payload["generationConfig"] = generation_config;
        }

        debug!("Sending request to Google Gemini: {}", context.request_id);

        // Use model name as-is (already in correct format from model listing)
        let response = self
            .client
            .post(&format!("{}/v1beta/models/{}:generateContent", self.base_url, request.model))
            .header("Content-Type", "application/json")
            .query(&[("key", &self.api_key)])
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Google Gemini API error: {}", error_text);
            return Err(OmenError::Provider(format!(
                "Google Gemini API error: {}",
                error_text
            )));
        }

        let gemini_response: serde_json::Value = response.json().await?;
        self.gemini_to_openai_response(
            gemini_response,
            &context.request_id.to_string(),
            &request.model,
        )
    }

    async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        let (system_instruction, contents) = self.openai_to_gemini_messages(&request.messages);

        let mut payload = json!({
            "contents": contents,
        });

        if !system_instruction.is_empty() {
            payload["systemInstruction"] = json!({
                "parts": [{"text": system_instruction}]
            });
        }

        // Add generation config
        let mut generation_config = json!({});
        if let Some(temp) = request.temperature {
            generation_config["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }
        if !generation_config.as_object().unwrap().is_empty() {
            payload["generationConfig"] = generation_config;
        }

        debug!("Sending streaming request to Google Gemini: {}", context.request_id);

        // Use model name as-is (already in correct format from model listing)
        let response = self
            .client
            .post(&format!("{}/v1beta/models/{}:streamGenerateContent", self.base_url, request.model))
            .header("Content-Type", "application/json")
            .query(&[("key", &self.api_key)])
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OmenError::Provider(format!(
                "Google Gemini API error: {}",
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
                        // Parse Gemini streaming format
                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let json_str = line.trim_start_matches("data: ");
                                if let Ok(gemini_chunk) = serde_json::from_str::<serde_json::Value>(json_str) {
                                    if let Some(candidates) = gemini_chunk["candidates"].as_array() {
                                        if let Some(candidate) = candidates.first() {
                                            if let Some(parts) = candidate["content"]["parts"].as_array() {
                                                if let Some(part) = parts.first() {
                                                    if let Some(text) = part["text"].as_str() {
                                                        let openai_chunk = ChatCompletionChunk {
                                                            id: request_id.clone(),
                                                            object: "chat.completion.chunk".to_string(),
                                                            created: chrono::Utc::now().timestamp(),
                                                            model: model.clone(),
                                                            choices: vec![ChatChoiceDelta {
                                                                index: 0,
                                                                delta: ChatMessageDelta {
                                                                    role: None,
                                                                    content: Some(text.to_string()),
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
                                    }
                                }
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