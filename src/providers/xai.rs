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
pub struct XaiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl XaiProvider {
    pub async fn new(
        api_key: String,
        base_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        let base_url = base_url.unwrap_or_else(|| "https://api.x.ai/v1".to_string());

        let provider = Self {
            client,
            api_key,
            base_url,
        };

        debug!("✅ xAI Grok provider initialized");

        Ok(provider)
    }
}

#[async_trait]
impl Provider for XaiProvider {
    fn id(&self) -> &str {
        "xai"
    }

    fn name(&self) -> &str {
        "xAI Grok"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Xai
    }

    async fn health_check(&self) -> Result<bool> {
        let response = self
            .client
            .get(&format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => {
                debug!("xAI health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        let response = self
            .client
            .get(&format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            // If models endpoint fails, return known models
            return Ok(vec![
                Model {
                    id: "grok-beta".to_string(),
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "xai".to_string(),
                    provider: "xai".to_string(),
                    context_length: 131072, // 128k tokens
                    pricing: ModelPricing {
                        input_per_1k: 0.0, // Pricing TBD by xAI
                        output_per_1k: 0.0,
                    },
                    capabilities: ModelCapabilities {
                        vision: false,
                        functions: true,
                        streaming: true,
                    },
                },
                Model {
                    id: "grok-vision-beta".to_string(),
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "xai".to_string(),
                    provider: "xai".to_string(),
                    context_length: 131072,
                    pricing: ModelPricing {
                        input_per_1k: 0.0,
                        output_per_1k: 0.0,
                    },
                    capabilities: ModelCapabilities {
                        vision: true,
                        functions: true,
                        streaming: true,
                    },
                },
            ]);
        }

        let data: serde_json::Value = response.json().await?;
        let mut models = Vec::new();

        if let Some(model_list) = data["data"].as_array() {
            for model_data in model_list {
                if let Some(model_id) = model_data["id"].as_str() {
                    let context_length = if model_id.contains("grok") { 131072 } else { 8192 };

                    models.push(Model {
                        id: model_id.to_string(),
                        object: "model".to_string(),
                        created: model_data["created"].as_i64().unwrap_or(0),
                        owned_by: "xai".to_string(),
                        provider: "xai".to_string(),
                        context_length,
                        pricing: ModelPricing {
                            input_per_1k: 0.0, // xAI pricing TBD
                            output_per_1k: 0.0,
                        },
                        capabilities: ModelCapabilities {
                            vision: model_id.contains("vision"),
                            functions: true,
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
        let mut payload = json!({
            "model": request.model,
            "messages": request.messages,
        });

        // Add optional parameters
        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            payload["max_tokens"] = json!(max_tokens);
        }
        if let Some(top_p) = request.top_p {
            payload["top_p"] = json!(top_p);
        }
        if let Some(freq_penalty) = request.frequency_penalty {
            payload["frequency_penalty"] = json!(freq_penalty);
        }
        if let Some(pres_penalty) = request.presence_penalty {
            payload["presence_penalty"] = json!(pres_penalty);
        }
        if let Some(ref stop) = request.stop {
            payload["stop"] = json!(stop);
        }
        if let Some(ref tools) = request.tools {
            payload["tools"] = json!(tools);
        }
        if let Some(ref tool_choice) = request.tool_choice {
            payload["tool_choice"] = json!(tool_choice);
        }

        debug!("Sending request to xAI Grok: {}", context.request_id);

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("xAI API error: {}", error_text);
            return Err(OmenError::Provider(format!(
                "xAI API error: {}",
                error_text
            )));
        }

        let xai_response: ChatCompletionResponse = response.json().await?;
        Ok(xai_response)
    }

    async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        let mut payload = json!({
            "model": request.model,
            "messages": request.messages,
            "stream": true,
        });

        // Add optional parameters
        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            payload["max_tokens"] = json!(max_tokens);
        }
        if let Some(top_p) = request.top_p {
            payload["top_p"] = json!(top_p);
        }

        debug!("Sending streaming request to xAI Grok: {}", context.request_id);

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OmenError::Provider(format!(
                "xAI API error: {}",
                error_text
            )));
        }

        let stream = response
            .bytes_stream()
            .map(|chunk| {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        // xAI uses OpenAI-compatible SSE format
                        Ok(text.to_string())
                    }
                    Err(e) => Err(OmenError::HttpClient(e)),
                }
            });

        Ok(Box::new(stream))
    }
}