use crate::{
    error::{OmenError, Result},
    router::OmenRouter,
    types::*,
};
use std::sync::Arc;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info};
use uuid::Uuid;

// Include the generated protobuf code
pub mod proto {
    tonic::include_proto!("omen.v1");
}

use proto::{
    omen_service_server::{OmenService, OmenServiceServer},
    ChatCompletionRequest as GrpcChatCompletionRequest,
    ChatCompletionResponse as GrpcChatCompletionResponse,
    ChatCompletionChunk as GrpcChatCompletionChunk,
    ListModelsRequest, ListModelsResponse,
    HealthCheckRequest, HealthCheckResponse,
    ProviderStatusRequest, ProviderStatusResponse,
    Model as GrpcModel, ProviderInfo,
    ChatMessage as GrpcChatMessage, ChatChoice as GrpcChatChoice,
    ChatChoiceDelta as GrpcChatChoiceDelta, ChatMessageDelta as GrpcChatMessageDelta,
    Usage as GrpcUsage, ModelPricing as GrpcModelPricing, ModelCapabilities as GrpcModelCapabilities,
    ToolCall as GrpcToolCall, Function as GrpcFunction,
};

pub struct OmenGrpcService {
    router: Arc<OmenRouter>,
}

impl OmenGrpcService {
    pub fn new(router: Arc<OmenRouter>) -> Self {
        Self { router }
    }

    pub fn into_service(self) -> OmenServiceServer<Self> {
        OmenServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl OmenService for OmenGrpcService {
    async fn chat_completion(
        &self,
        request: Request<ChatCompletionRequest>,
    ) -> std::result::Result<Response<ChatCompletionResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC chat completion request for model: {}", req.model);

        // Convert protobuf request to internal types
        let chat_request = convert_grpc_to_chat_request(req)?;
        let context = create_request_context();

        // Process the request
        match self.router.chat_completion(chat_request, context).await {
            Ok(response) => {
                let grpc_response = convert_chat_response_to_grpc(response)?;
                Ok(Response::new(grpc_response))
            }
            Err(e) => {
                error!("gRPC chat completion failed: {}", e);
                Err(Status::internal(format!("Chat completion failed: {}", e)))
            }
        }
    }

    type StreamChatCompletionStream = tokio_stream::wrappers::ReceiverStream<
        std::result::Result<ChatCompletionChunk, Status>,
    >;

    async fn stream_chat_completion(
        &self,
        request: Request<ChatCompletionRequest>,
    ) -> std::result::Result<Response<Self::StreamChatCompletionStream>, Status> {
        let req = request.into_inner();
        debug!("gRPC streaming chat completion request for model: {}", req.model);

        // Convert protobuf request to internal types
        let mut chat_request = convert_grpc_to_chat_request(req)?;
        chat_request.stream = true;
        let context = create_request_context();

        // Create a channel for streaming responses
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Process the streaming request
        let router = self.router.clone();
        tokio::spawn(async move {
            match router.stream_chat_completion(chat_request, context).await {
                Ok(mut stream) => {
                    use futures::StreamExt;
                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk_text) => {
                                // Parse SSE chunk and convert to gRPC
                                if let Ok(grpc_chunk) = parse_sse_to_grpc_chunk(&chunk_text) {
                                    if tx.send(Ok(grpc_chunk)).await.is_err() {
                                        break; // Client disconnected
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(Err(Status::internal(format!("Stream error: {}", e)))).await;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Status::internal(format!("Failed to start stream: {}", e)))).await;
                }
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Response::new(stream))
    }

    async fn list_models(
        &self,
        _request: Request<ListModelsRequest>,
    ) -> std::result::Result<Response<ListModelsResponse>, Status> {
        debug!("gRPC list models request");

        match self.router.list_models().await {
            Ok(models) => {
                let grpc_models = models
                    .into_iter()
                    .map(convert_model_to_grpc)
                    .collect();

                let response = ListModelsResponse {
                    object: "list".to_string(),
                    data: grpc_models,
                };

                Ok(Response::new(response))
            }
            Err(e) => {
                error!("gRPC list models failed: {}", e);
                Err(Status::internal(format!("Failed to list models: {}", e)))
            }
        }
    }

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> std::result::Result<Response<HealthCheckResponse>, Status> {
        debug!("gRPC health check request");

        let providers = self.router.list_providers().await;
        let mut provider_health = std::collections::HashMap::new();
        let mut healthy_count = 0;

        for provider in &providers {
            match provider.health_check().await {
                Ok(healthy) => {
                    provider_health.insert(provider.id().to_string(), healthy);
                    if healthy {
                        healthy_count += 1;
                    }
                }
                Err(_) => {
                    provider_health.insert(provider.id().to_string(), false);
                }
            }
        }

        let response = HealthCheckResponse {
            status: if healthy_count > 0 { "healthy" } else { "unhealthy" }.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            providers_available: providers.len() as i32,
            provider_health,
        };

        Ok(Response::new(response))
    }

    async fn get_provider_status(
        &self,
        _request: Request<ProviderStatusRequest>,
    ) -> std::result::Result<Response<ProviderStatusResponse>, Status> {
        debug!("gRPC provider status request");

        let providers = self.router.list_providers().await;
        let mut provider_infos = Vec::new();

        for provider in providers {
            let healthy = provider.health_check().await.unwrap_or(false);
            let models = provider.list_models().await.unwrap_or_default();

            provider_infos.push(ProviderInfo {
                id: provider.id().to_string(),
                name: provider.name().to_string(),
                r#type: format!("{:?}", provider.provider_type()),
                enabled: true, // If it's in the list, it's enabled
                healthy,
                models_count: models.len() as i32,
                last_error: None, // TODO: Track last error
            });
        }

        let response = ProviderStatusResponse {
            providers: provider_infos,
        };

        Ok(Response::new(response))
    }
}

// Helper functions for conversion

fn convert_grpc_to_chat_request(req: ChatCompletionRequest) -> std::result::Result<crate::types::ChatCompletionRequest, Status> {
    let messages = req.messages
        .into_iter()
        .map(|msg| crate::types::ChatMessage {
            role: msg.role,
            content: msg.content,
            name: msg.name,
            tool_calls: msg.tool_calls.into_iter().map(|tc| crate::types::ToolCall {
                id: tc.id,
                r#type: tc.r#type,
                function: crate::types::Function {
                    name: tc.function.map(|f| f.name).unwrap_or_default(),
                    arguments: "{}".to_string(), // TODO: Convert from protobuf Value
                },
            }).collect(),
            tool_call_id: msg.tool_call_id,
        })
        .collect();

    Ok(crate::types::ChatCompletionRequest {
        model: req.model,
        messages,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        top_p: req.top_p,
        frequency_penalty: req.frequency_penalty,
        presence_penalty: req.presence_penalty,
        stop: if req.stop.is_empty() { None } else { Some(req.stop) },
        stream: req.stream,
        tools: None, // TODO: Convert tools
        tool_choice: None, // TODO: Convert tool choice
    })
}

fn convert_chat_response_to_grpc(resp: crate::types::ChatCompletionResponse) -> std::result::Result<ChatCompletionResponse, Status> {
    let choices = resp.choices
        .into_iter()
        .map(|choice| ChatChoice {
            index: choice.index,
            message: Some(ChatMessage {
                role: choice.message.role,
                content: choice.message.content,
                name: choice.message.name,
                tool_calls: choice.message.tool_calls.unwrap_or_default()
                    .into_iter()
                    .map(|tc| ToolCall {
                        id: tc.id,
                        r#type: tc.r#type,
                        function: Some(Function {
                            name: tc.function.name,
                            description: None,
                            parameters: std::collections::HashMap::new(), // TODO: Parse arguments
                        }),
                    })
                    .collect(),
                tool_call_id: choice.message.tool_call_id,
            }),
            finish_reason: choice.finish_reason,
        })
        .collect();

    Ok(ChatCompletionResponse {
        id: resp.id,
        object: resp.object,
        created: resp.created,
        model: resp.model,
        choices,
        usage: Some(Usage {
            prompt_tokens: resp.usage.prompt_tokens,
            completion_tokens: resp.usage.completion_tokens,
            total_tokens: resp.usage.total_tokens,
        }),
        system_fingerprint: resp.system_fingerprint,
    })
}

fn convert_model_to_grpc(model: crate::types::Model) -> Model {
    Model {
        id: model.id,
        object: model.object,
        created: model.created,
        owned_by: model.owned_by,
        provider: model.provider,
        context_length: model.context_length,
        pricing: Some(ModelPricing {
            input_per_1k: model.pricing.input_per_1k,
            output_per_1k: model.pricing.output_per_1k,
        }),
        capabilities: Some(ModelCapabilities {
            vision: model.capabilities.vision,
            functions: model.capabilities.functions,
            streaming: model.capabilities.streaming,
        }),
    }
}

fn create_request_context() -> crate::types::RequestContext {
    crate::types::RequestContext {
        request_id: Uuid::new_v4(),
        user_id: None,
        api_key: None,
        intent: None,
        tags: std::collections::HashMap::new(),
    }
}

fn parse_sse_to_grpc_chunk(sse_text: &str) -> std::result::Result<ChatCompletionChunk, Status> {
    // Parse SSE format: "data: {json}\n\n"
    for line in sse_text.lines() {
        if line.starts_with("data: ") {
            let json_str = line.trim_start_matches("data: ");
            if json_str == "[DONE]" {
                // End of stream marker - could send a special chunk
                continue;
            }

            match serde_json::from_str::<crate::types::ChatCompletionChunk>(json_str) {
                Ok(chunk) => {
                    return Ok(ChatCompletionChunk {
                        id: chunk.id,
                        object: chunk.object,
                        created: chunk.created,
                        model: chunk.model,
                        choices: chunk.choices
                            .into_iter()
                            .map(|choice| ChatChoiceDelta {
                                index: choice.index,
                                delta: Some(ChatMessageDelta {
                                    role: choice.delta.role,
                                    content: choice.delta.content,
                                    tool_calls: vec![], // TODO: Convert tool calls
                                }),
                                finish_reason: choice.finish_reason,
                            })
                            .collect(),
                        system_fingerprint: chunk.system_fingerprint,
                    });
                }
                Err(e) => {
                    error!("Failed to parse SSE chunk: {}", e);
                }
            }
        }
    }

    Err(Status::internal("Invalid SSE chunk format"))
}

pub async fn start_grpc_server(
    router: Arc<OmenRouter>,
    addr: std::net::SocketAddr,
) -> Result<()> {
    let service = OmenGrpcService::new(router);

    info!("🚀 Starting OMEN gRPC server on {}", addr);

    tonic::transport::Server::builder()
        .add_service(service.into_service())
        .serve(addr)
        .await
        .map_err(|e| OmenError::Server(format!("gRPC server error: {}", e)))?;

    Ok(())
}