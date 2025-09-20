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
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAIProvider {
    pub async fn new(
        api_key: String,
        base_url: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let provider = Self {
            client,
            api_key,
            base_url,
        };

        // Test the connection
        if provider.health_check().await? {
            debug!("✅ OpenAI provider initialized successfully");
        } else {
            warn!("⚠️  OpenAI provider initialized but health check failed");
        }

        Ok(provider)
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
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
                debug!("OpenAI health check failed: {}", e);
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
            return Err(OmenError::Provider(format!(
                "OpenAI API error: {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response.json().await?;
        let mut models = Vec::new();

        if let Some(model_list) = data["data"].as_array() {
            for model_data in model_list {
                if let Some(model_id) = model_data["id"].as_str() {
                    // Only include chat models
                    if model_id.starts_with("gpt-") {
                        let (input_price, output_price) = get_openai_pricing(model_id);
                        let context_length = get_openai_context_length(model_id);

                        models.push(Model {
                            id: model_id.to_string(),
                            object: "model".to_string(),
                            created: model_data["created"].as_i64().unwrap_or(0),
                            owned_by: "openai".to_string(),
                            provider: "openai".to_string(),
                            context_length,
                            pricing: ModelPricing {
                                input_per_1k: input_price,
                                output_per_1k: output_price,
                            },
                            capabilities: ModelCapabilities {
                                vision: model_id.contains("vision") || model_id.contains("gpt-4"),
                                functions: true,
                                streaming: true,
                            },
                        });
                    }
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
        let payload = self.build_openai_request(request);

        debug!("Sending request to OpenAI: {}", context.request_id);

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
            error!("OpenAI API error: {}", error_text);
            return Err(OmenError::Provider(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let openai_response: ChatCompletionResponse = response.json().await?;
        Ok(openai_response)
    }

    async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        let mut payload = self.build_openai_request(request);
        payload["stream"] = json!(true);

        debug!("Sending streaming request to OpenAI: {}", context.request_id);

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
                "OpenAI API error: {}",
                error_text
            )));
        }

        let stream = response
            .bytes_stream()
            .map(|chunk| {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        // Parse SSE format
                        if text.starts_with("data: ") {
                            let json_str = text.trim_start_matches("data: ").trim();
                            if json_str == "[DONE]" {
                                return Ok("data: [DONE]\n\n".to_string());
                            }
                            if let Ok(chunk_data) = serde_json::from_str::<ChatCompletionChunk>(json_str) {
                                return Ok(format!("data: {}\n\n", serde_json::to_string(&chunk_data).unwrap_or_default()));
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

impl OpenAIProvider {
    fn build_openai_request(&self, request: &ChatCompletionRequest) -> serde_json::Value {
        let mut payload = json!({
            "model": request.model,
            "messages": request.messages,
        });

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

        payload
    }
}

fn get_openai_pricing(model: &str) -> (f64, f64) {
    match model {
        "gpt-4" | "gpt-4-0613" => (0.03, 0.06),
        "gpt-4-32k" | "gpt-4-32k-0613" => (0.06, 0.12),
        "gpt-4-turbo" | "gpt-4-turbo-preview" => (0.01, 0.03),
        "gpt-4o" => (0.005, 0.015),
        "gpt-4o-mini" => (0.00015, 0.0006),
        "gpt-3.5-turbo" | "gpt-3.5-turbo-0125" => (0.0005, 0.0015),
        "gpt-3.5-turbo-instruct" => (0.0015, 0.002),
        _ => (0.001, 0.002), // Default pricing
    }
}

fn get_openai_context_length(model: &str) -> u32 {
    match model {
        "gpt-4" | "gpt-4-0613" => 8192,
        "gpt-4-32k" | "gpt-4-32k-0613" => 32768,
        "gpt-4-turbo" | "gpt-4-turbo-preview" => 128000,
        "gpt-4o" | "gpt-4o-mini" => 128000,
        "gpt-3.5-turbo" | "gpt-3.5-turbo-0125" => 16385,
        "gpt-3.5-turbo-instruct" => 4096,
        _ => 4096, // Default context length
    }
}