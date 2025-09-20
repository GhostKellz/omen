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
pub struct BedrockProvider {
    client: Client,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

impl BedrockProvider {
    pub async fn new(
        region: String,
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        let provider = Self {
            client,
            region,
            access_key_id,
            secret_access_key,
            session_token,
        };

        debug!("✅ AWS Bedrock provider initialized");

        Ok(provider)
    }

    fn sign_request(&self, _payload: &str) -> Result<String> {
        // TODO: Implement AWS Signature Version 4
        // For now, return placeholder authorization header
        Ok(format!("AWS4-HMAC-SHA256 Credential={}/20231201/{}/bedrock/aws4_request",
                  self.access_key_id, self.region))
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    fn id(&self) -> &str {
        "bedrock"
    }

    fn name(&self) -> &str {
        "AWS Bedrock"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Bedrock
    }

    async fn health_check(&self) -> Result<bool> {
        // Simple ping to Bedrock runtime
        let response = self
            .client
            .get(&format!("https://bedrock-runtime.{}.amazonaws.com/foundation-models", self.region))
            .header("Authorization", self.sign_request("")?)
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().as_u16() != 500),
            Err(e) => {
                debug!("AWS Bedrock health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        // AWS Bedrock known models
        Ok(vec![
            Model {
                id: "anthropic.claude-3-opus-20240229-v1:0".to_string(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "anthropic".to_string(),
                provider: "bedrock".to_string(),
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
                id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "anthropic".to_string(),
                provider: "bedrock".to_string(),
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
                id: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "anthropic".to_string(),
                provider: "bedrock".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    input_per_1k: 0.00025,
                    output_per_1k: 0.00125,
                },
                capabilities: ModelCapabilities {
                    vision: false,
                    functions: true,
                    streaming: true,
                },
            },
            Model {
                id: "amazon.titan-text-premier-v1:0".to_string(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "amazon".to_string(),
                provider: "bedrock".to_string(),
                context_length: 32000,
                pricing: ModelPricing {
                    input_per_1k: 0.0005,
                    output_per_1k: 0.0015,
                },
                capabilities: ModelCapabilities {
                    vision: false,
                    functions: false,
                    streaming: true,
                },
            },
            Model {
                id: "meta.llama3-70b-instruct-v1:0".to_string(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "meta".to_string(),
                provider: "bedrock".to_string(),
                context_length: 8192,
                pricing: ModelPricing {
                    input_per_1k: 0.00265,
                    output_per_1k: 0.0035,
                },
                capabilities: ModelCapabilities {
                    vision: false,
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
        // Convert OpenAI format to Bedrock format based on model
        let bedrock_payload = if request.model.contains("anthropic.claude") {
            self.convert_to_claude_bedrock_format(request)?
        } else if request.model.contains("amazon.titan") {
            self.convert_to_titan_format(request)?
        } else if request.model.contains("meta.llama") {
            self.convert_to_llama_format(request)?
        } else {
            return Err(OmenError::ModelNotFound(request.model.clone()));
        };

        debug!("Sending request to AWS Bedrock: {}", context.request_id);

        let authorization = self.sign_request(&serde_json::to_string(&bedrock_payload)?)?;

        let response = self
            .client
            .post(&format!(
                "https://bedrock-runtime.{}.amazonaws.com/model/{}/invoke",
                self.region, request.model
            ))
            .header("Authorization", authorization)
            .header("Content-Type", "application/json")
            .json(&bedrock_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("AWS Bedrock API error: {}", error_text);
            return Err(OmenError::Provider(format!(
                "AWS Bedrock API error: {}",
                error_text
            )));
        }

        let bedrock_response: serde_json::Value = response.json().await?;
        self.convert_bedrock_to_openai(bedrock_response, context, &request.model)
    }

    async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
        context: &RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        // TODO: Implement Bedrock streaming with invoke-with-response-stream
        let mut payload = json!({
            "model": request.model,
            "messages": request.messages,
            "stream": true,
        });

        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            payload["max_tokens"] = json!(max_tokens);
        }

        debug!("Sending streaming request to AWS Bedrock: {}", context.request_id);

        let authorization = self.sign_request(&serde_json::to_string(&payload)?)?;

        let response = self
            .client
            .post(&format!(
                "https://bedrock-runtime.{}.amazonaws.com/model/{}/invoke-with-response-stream",
                self.region, request.model
            ))
            .header("Authorization", authorization)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OmenError::Provider(format!(
                "AWS Bedrock API error: {}",
                error_text
            )));
        }

        let stream = response
            .bytes_stream()
            .map(|chunk| {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        // AWS Bedrock uses event-stream format
                        Ok(text.to_string())
                    }
                    Err(e) => Err(OmenError::HttpClient(e)),
                }
            });

        Ok(Box::new(stream))
    }
}

impl BedrockProvider {
    fn convert_to_claude_bedrock_format(&self, request: &ChatCompletionRequest) -> Result<serde_json::Value> {
        // Convert OpenAI messages to Claude Bedrock format
        let mut anthropic_messages = Vec::new();
        let mut system_prompt = String::new();

        for msg in &request.messages {
            match msg.role.as_str() {
                "system" => {
                    if !system_prompt.is_empty() {
                        system_prompt.push('\n');
                    }
                    system_prompt.push_str(&msg.content.text());
                }
                "user" | "assistant" => {
                    anthropic_messages.push(json!({
                        "role": msg.role,
                        "content": msg.content
                    }));
                }
                _ => {} // Skip other roles
            }
        }

        let mut payload = json!({
            "messages": anthropic_messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "anthropic_version": "bedrock-2023-05-31"
        });

        if !system_prompt.is_empty() {
            payload["system"] = json!(system_prompt);
        }

        if let Some(temp) = request.temperature {
            payload["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            payload["top_p"] = json!(top_p);
        }

        Ok(payload)
    }

    fn convert_to_titan_format(&self, request: &ChatCompletionRequest) -> Result<serde_json::Value> {
        // Amazon Titan format
        let prompt = request.messages
            .iter()
            .map(|msg| format!("{}: {}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(json!({
            "inputText": prompt,
            "textGenerationConfig": {
                "maxTokenCount": request.max_tokens.unwrap_or(4096),
                "temperature": request.temperature.unwrap_or(0.7),
                "topP": request.top_p.unwrap_or(0.9)
            }
        }))
    }

    fn convert_to_llama_format(&self, request: &ChatCompletionRequest) -> Result<serde_json::Value> {
        // Meta Llama format
        let prompt = request.messages
            .iter()
            .map(|msg| format!("<|{}|>{}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("");

        Ok(json!({
            "prompt": prompt,
            "max_gen_len": request.max_tokens.unwrap_or(4096),
            "temperature": request.temperature.unwrap_or(0.7),
            "top_p": request.top_p.unwrap_or(0.9)
        }))
    }

    fn convert_bedrock_to_openai(
        &self,
        bedrock_response: serde_json::Value,
        context: &RequestContext,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let content = if model.contains("anthropic.claude") {
            bedrock_response["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string()
        } else if model.contains("amazon.titan") {
            bedrock_response["results"][0]["outputText"]
                .as_str()
                .unwrap_or("")
                .to_string()
        } else if model.contains("meta.llama") {
            bedrock_response["generation"]
                .as_str()
                .unwrap_or("")
                .to_string()
        } else {
            return Err(OmenError::ModelNotFound(model.to_string()));
        };

        // Bedrock doesn't always provide token counts
        let usage = if let Some(usage_data) = bedrock_response.get("usage") {
            Usage {
                prompt_tokens: usage_data["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_data["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_data["input_tokens"].as_u64().unwrap_or(0) as u32
                    + usage_data["output_tokens"].as_u64().unwrap_or(0) as u32,
            }
        } else {
            Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            }
        };

        Ok(ChatCompletionResponse {
            id: context.request_id.to_string(),
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
            usage,
            system_fingerprint: None,
        })
    }
}