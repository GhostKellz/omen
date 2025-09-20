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
pub struct AzureProvider {
    client: Client,
    endpoint: String,
    api_key: String,
    api_version: String,
}

impl AzureProvider {
    pub async fn new(
        endpoint: String,
        api_key: String,
        api_version: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        let api_version = api_version.unwrap_or_else(|| "2024-02-15-preview".to_string());

        let provider = Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            api_key,
            api_version,
        };

        debug!("✅ Azure OpenAI provider initialized");

        Ok(provider)
    }
}

#[async_trait]
impl Provider for AzureProvider {
    fn id(&self) -> &str {
        "azure"
    }

    fn name(&self) -> &str {
        "Azure OpenAI"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Azure
    }

    async fn health_check(&self) -> Result<bool> {
        // Try to list deployments to check connectivity
        let response = self
            .client
            .get(&format!("{}/openai/deployments", self.endpoint))
            .header("api-key", &self.api_key)
            .query(&[("api-version", &self.api_version)])
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => {
                debug!("Azure OpenAI health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        // Get Azure deployments (which are models in Azure OpenAI)
        let response = self
            .client
            .get(&format!("{}/openai/deployments", self.endpoint))
            .header("api-key", &self.api_key)
            .query(&[("api-version", &self.api_version)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(OmenError::Provider(format!(
                "Azure OpenAI API error: {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response.json().await?;
        let mut models = Vec::new();

        if let Some(deployments) = data["data"].as_array() {
            for deployment in deployments {
                if let Some(model_name) = deployment["model"].as_str() {
                    let deployment_id = deployment["id"].as_str().unwrap_or(model_name);

                    let (input_price, output_price) = get_azure_pricing(model_name);
                    let context_length = get_azure_context_length(model_name);

                    models.push(Model {
                        id: deployment_id.to_string(),
                        object: "model".to_string(),
                        created: deployment["created_at"].as_i64().unwrap_or(0),
                        owned_by: "microsoft".to_string(),
                        provider: "azure".to_string(),
                        context_length,
                        pricing: ModelPricing {
                            input_per_1k: input_price,
                            output_per_1k: output_price,
                        },
                        capabilities: ModelCapabilities {
                            vision: model_name.contains("vision") || model_name.contains("gpt-4"),
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

        debug!("Sending request to Azure OpenAI: {}", context.request_id);

        let response = self
            .client
            .post(&format!(
                "{}/openai/deployments/{}/chat/completions",
                self.endpoint, request.model
            ))
            .header("api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .query(&[("api-version", &self.api_version)])
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Azure OpenAI API error: {}", error_text);
            return Err(OmenError::Provider(format!(
                "Azure OpenAI API error: {}",
                error_text
            )));
        }

        let azure_response: ChatCompletionResponse = response.json().await?;
        Ok(azure_response)
    }

    async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        let mut payload = json!({
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

        debug!("Sending streaming request to Azure OpenAI: {}", context.request_id);

        let response = self
            .client
            .post(&format!(
                "{}/openai/deployments/{}/chat/completions",
                self.endpoint, request.model
            ))
            .header("api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .query(&[("api-version", &self.api_version)])
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OmenError::Provider(format!(
                "Azure OpenAI API error: {}",
                error_text
            )));
        }

        let stream = response
            .bytes_stream()
            .map(|chunk| {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        // Azure OpenAI uses the same SSE format as OpenAI
                        Ok(text.to_string())
                    }
                    Err(e) => Err(OmenError::HttpClient(e)),
                }
            });

        Ok(Box::new(stream))
    }
}

fn get_azure_pricing(model: &str) -> (f64, f64) {
    // Azure OpenAI pricing (varies by region, these are US East prices)
    match model {
        m if m.contains("gpt-4") && m.contains("32k") => (0.06, 0.12),
        m if m.contains("gpt-4-turbo") => (0.01, 0.03),
        m if m.contains("gpt-4") => (0.03, 0.06),
        m if m.contains("gpt-35-turbo") || m.contains("gpt-3.5-turbo") => (0.0015, 0.002),
        _ => (0.001, 0.002), // Default pricing
    }
}

fn get_azure_context_length(model: &str) -> u32 {
    match model {
        m if m.contains("32k") => 32768,
        m if m.contains("gpt-4-turbo") => 128000,
        m if m.contains("gpt-4") => 8192,
        m if m.contains("gpt-35-turbo") || m.contains("gpt-3.5-turbo") => 16385,
        _ => 4096, // Default context length
    }
}