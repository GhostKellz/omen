use crate::{
    error::{OmenError, Result},
    providers::Provider,
    types::*,
};
use futures::{stream::Stream, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio::{
    select,
    sync::mpsc,
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub enum MultiplexStrategy {
    Single,
    Race { k: usize },
    SpeculateK { k: usize, delay_ms: u64 },
    ParallelMerge { k: usize },
}

impl Default for MultiplexStrategy {
    fn default() -> Self {
        MultiplexStrategy::Race { k: 2 }
    }
}

impl From<&OmenConfig> for MultiplexStrategy {
    fn from(config: &OmenConfig) -> Self {
        match config.strategy.as_deref() {
            Some("single") => MultiplexStrategy::Single,
            Some("race") => MultiplexStrategy::Race {
                k: config.k.unwrap_or(2) as usize,
            },
            Some("speculate_k") => MultiplexStrategy::SpeculateK {
                k: config.k.unwrap_or(2) as usize,
                delay_ms: 150, // Default speculative delay
            },
            Some("parallel_merge") => MultiplexStrategy::ParallelMerge {
                k: config.k.unwrap_or(2) as usize,
            },
            _ => MultiplexStrategy::default(),
        }
    }
}

/// Stream event types for multiplexing - all variants are part of the public API
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Token {
        provider_id: String,
        chunk: String,
        latency_ms: u64,
    },
    Error {
        provider_id: String,
        error: String,
    },
    Done {
        provider_id: String,
        total_tokens: u32,
        cost_usd: f64,
    },
    Upgrade {
        from_provider: String,
        to_provider: String,
        reason: String,
    },
}

#[allow(dead_code)]
pub struct StreamMultiplexer {
    providers: Vec<Arc<dyn Provider>>,
    budget_cap: f64,
    max_latency: Duration,
    min_useful_tokens: usize,
    cancellation_token: CancellationToken,
}

impl StreamMultiplexer {
    pub fn new(
        providers: Vec<Arc<dyn Provider>>,
        config: &OmenConfig,
    ) -> Self {
        Self {
            providers,
            budget_cap: config.budget_usd.unwrap_or(0.10),
            max_latency: Duration::from_millis(config.max_latency_ms.unwrap_or(3000) as u64),
            min_useful_tokens: config.min_useful_tokens.unwrap_or(5) as usize,
            cancellation_token: CancellationToken::new(),
        }
    }

    pub async fn multiplex_stream(
        &self,
        request: ChatCompletionRequest,
        context: RequestContext,
        strategy: MultiplexStrategy,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        match strategy {
            MultiplexStrategy::Single => self.run_single(request, context).await,
            MultiplexStrategy::Race { k } => self.run_race(request, context, k).await,
            MultiplexStrategy::SpeculateK { k, delay_ms } => {
                self.run_speculate_k(request, context, k, delay_ms).await
            }
            MultiplexStrategy::ParallelMerge { k } => {
                self.run_parallel_merge(request, context, k).await
            }
        }
    }

    async fn run_single(
        &self,
        request: ChatCompletionRequest,
        context: RequestContext,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        if let Some(provider) = self.providers.first() {
            info!("🎯 Single strategy: using provider {}", provider.name());
            provider.stream_chat_completion(&request, &context).await
        } else {
            Err(OmenError::ProviderUnavailable("No providers available".to_string()))
        }
    }

    async fn run_race(
        &self,
        request: ChatCompletionRequest,
        context: RequestContext,
        k: usize,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        info!("🏁 Race strategy: {} providers racing to first token", k.min(self.providers.len()));

        let candidates = self.providers.iter().take(k).cloned().collect::<Vec<_>>();
        self.race_providers(request, context, candidates).await
    }

    async fn run_speculate_k(
        &self,
        request: ChatCompletionRequest,
        context: RequestContext,
        k: usize,
        delay_ms: u64,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        info!("⚡ Speculative strategy: starting local immediately, cloud after {}ms", delay_ms);

        // Start local provider immediately
        let local_providers = self.providers.iter()
            .filter(|p| p.id() == "ollama")
            .take(1)
            .cloned()
            .collect::<Vec<_>>();

        if local_providers.is_empty() {
            // Fallback to race if no local provider
            return self.run_race(request, context, k).await;
        }

        // Start cloud providers with delay
        let cloud_providers = self.providers.iter()
            .filter(|p| p.id() != "ollama")
            .take(k - 1)
            .cloned()
            .collect::<Vec<_>>();

        self.speculate_with_delay(request, context, local_providers, cloud_providers, delay_ms).await
    }

    async fn run_parallel_merge(
        &self,
        request: ChatCompletionRequest,
        context: RequestContext,
        k: usize,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        info!("🔀 Parallel merge strategy: {} providers with quality merging", k.min(self.providers.len()));

        // This is the most complex strategy - merge multiple streams
        // For now, fallback to race (can be enhanced later)
        self.run_race(request, context, k).await
    }

    async fn race_providers(
        &self,
        request: ChatCompletionRequest,
        context: RequestContext,
        candidates: Vec<Arc<dyn Provider>>,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(100);
        let cancellation_token = CancellationToken::new();
        let mut provider_handles = Vec::new();

        // Start all providers concurrently
        for provider in candidates {
            let _provider_id = provider.id().to_string();
            let req_clone = request.clone();
            let ctx_clone = context.clone();
            let tx_clone = tx.clone();
            let cancel_token = cancellation_token.child_token();
            let start_time = Instant::now();

            let handle = tokio::spawn(async move {
                Self::stream_provider_with_events(
                    provider,
                    req_clone,
                    ctx_clone,
                    tx_clone,
                    cancel_token,
                    start_time,
                ).await;
            });

            provider_handles.push(handle);
        }

        // Create the multiplexed stream
        let (stream_tx, stream_rx) = mpsc::channel::<Result<String>>(100);
        let cancellation_clone = cancellation_token.clone();
        let budget_cap = self.budget_cap;
        let max_latency = self.max_latency;
        let min_useful_tokens = self.min_useful_tokens;

        tokio::spawn(async move {
            let mut winner: Option<String> = None;
            let mut total_cost = 0.0;
            let mut token_count = 0;
            let race_start = Instant::now();

            // Apply global timeout
            let timeout_future = tokio::time::sleep(max_latency);
            tokio::pin!(timeout_future);

            loop {
                select! {
                    event = rx.recv() => {
                        match event {
                            Some(StreamEvent::Token { provider_id, chunk, .. }) => {
                                if Self::is_useful_token(&chunk, min_useful_tokens) && winner.is_none() {
                                    winner = Some(provider_id.clone());
                                    info!("🏆 Provider {} wins the race!", provider_id);
                                    cancellation_clone.cancel();
                                }

                                if winner.as_ref() == Some(&provider_id) {
                                    token_count += 1;
                                    let _ = stream_tx.send(Ok(chunk)).await;
                                }
                            }
                            Some(StreamEvent::Done { provider_id, cost_usd, .. }) => {
                                if winner.as_ref() == Some(&provider_id) {
                                    total_cost += cost_usd;
                                    let elapsed = race_start.elapsed();
                                    info!("✅ Race completed in {}ms, cost: ${:.4}", elapsed.as_millis(), total_cost);
                                    break;
                                }
                            }
                            Some(StreamEvent::Error { provider_id, error }) => {
                                if winner.as_ref() == Some(&provider_id) {
                                    warn!("❌ Winning provider {} failed: {}", provider_id, error);
                                    let _ = stream_tx.send(Err(OmenError::Provider(error))).await;
                                    break;
                                }
                            }
                            None => break,
                            _ => {}
                        }

                        // Budget check
                        if total_cost > budget_cap {
                            warn!("💰 Budget cap (${:.2}) exceeded, canceling race", budget_cap);
                            cancellation_clone.cancel();
                            break;
                        }
                    }
                    _ = &mut timeout_future => {
                        warn!("⏰ Race timeout ({}ms) exceeded", max_latency.as_millis());
                        cancellation_clone.cancel();
                        break;
                    }
                }
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(stream_rx);
        Ok(Box::new(stream))
    }

    async fn speculate_with_delay(
        &self,
        request: ChatCompletionRequest,
        context: RequestContext,
        local_providers: Vec<Arc<dyn Provider>>,
        cloud_providers: Vec<Arc<dyn Provider>>,
        delay_ms: u64,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>> {
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(100);
        let cancellation_token = CancellationToken::new();

        // Start local provider immediately
        for provider in local_providers {
            let _provider_id = provider.id().to_string();
            let req_clone = request.clone();
            let ctx_clone = context.clone();
            let tx_clone = tx.clone();
            let cancel_token = cancellation_token.child_token();
            let start_time = Instant::now();

            tokio::spawn(async move {
                Self::stream_provider_with_events(
                    provider,
                    req_clone,
                    ctx_clone,
                    tx_clone,
                    cancel_token,
                    start_time,
                ).await;
            });
        }

        // Start cloud providers with delay
        let cloud_tx = tx.clone();
        let cloud_cancel = cancellation_token.child_token();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;

            for provider in cloud_providers {
                let _provider_id = provider.id().to_string();
                let req_clone = request.clone();
                let ctx_clone = context.clone();
                let tx_clone = cloud_tx.clone();
                let cancel_token = cloud_cancel.child_token();
                let start_time = Instant::now();

                tokio::spawn(async move {
                    Self::stream_provider_with_events(
                        provider,
                        req_clone,
                        ctx_clone,
                        tx_clone,
                        cancel_token,
                        start_time,
                    ).await;
                });
            }
        });

        // Create the speculative stream (similar to race but with upgrade logic)
        let (stream_tx, stream_rx) = mpsc::channel::<Result<String>>(100);
        let cancellation_clone = cancellation_token.clone();

        tokio::spawn(async move {
            let mut current_provider: Option<String> = None;
            let mut can_upgrade = true;

            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::Token { provider_id, chunk, .. } => {
                        if current_provider.is_none() {
                            current_provider = Some(provider_id.clone());
                            info!("🚀 Speculative start with provider {}", provider_id);
                        } else if can_upgrade && provider_id != *current_provider.as_ref().unwrap() {
                            // Check if this is a quality upgrade
                            if Self::should_upgrade(&chunk) {
                                info!("⬆️ Upgrading from {} to {}", current_provider.as_ref().unwrap(), provider_id);
                                current_provider = Some(provider_id.clone());
                                can_upgrade = false; // Only upgrade once per request
                            }
                        }

                        if current_provider.as_ref() == Some(&provider_id) {
                            let _ = stream_tx.send(Ok(chunk)).await;
                        }
                    }
                    StreamEvent::Done { provider_id, .. } => {
                        if current_provider.as_ref() == Some(&provider_id) {
                            cancellation_clone.cancel();
                            break;
                        }
                    }
                    StreamEvent::Error { provider_id, error } => {
                        if current_provider.as_ref() == Some(&provider_id) {
                            let _ = stream_tx.send(Err(OmenError::Provider(error))).await;
                            break;
                        }
                    }
                    _ => {}
                }
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(stream_rx);
        Ok(Box::new(stream))
    }

    async fn stream_provider_with_events(
        provider: Arc<dyn Provider>,
        request: ChatCompletionRequest,
        context: RequestContext,
        tx: mpsc::Sender<StreamEvent>,
        cancel_token: CancellationToken,
        start_time: Instant,
    ) {
        let provider_id = provider.id().to_string();

        match provider.stream_chat_completion(&request, &context).await {
            Ok(mut stream) => {
                let mut total_tokens = 0;

                loop {
                    select! {
                        chunk_result = stream.next() => {
                            match chunk_result {
                                Some(Ok(chunk)) => {
                                    total_tokens += 1;
                                    let latency = start_time.elapsed().as_millis() as u64;

                                    let _ = tx.send(StreamEvent::Token {
                                        provider_id: provider_id.clone(),
                                        chunk,
                                        latency_ms: latency,
                                    }).await;
                                }
                                Some(Err(e)) => {
                                    let _ = tx.send(StreamEvent::Error {
                                        provider_id: provider_id.clone(),
                                        error: e.to_string(),
                                    }).await;
                                    break;
                                }
                                None => {
                                    // Stream finished
                                    let _ = tx.send(StreamEvent::Done {
                                        provider_id: provider_id.clone(),
                                        total_tokens,
                                        cost_usd: 0.01, // TODO: Calculate actual cost
                                    }).await;
                                    break;
                                }
                            }
                        }
                        _ = cancel_token.cancelled() => {
                            debug!("🛑 Provider {} cancelled", provider_id);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(StreamEvent::Error {
                    provider_id: provider_id.clone(),
                    error: e.to_string(),
                }).await;
            }
        }
    }

    fn is_useful_token(chunk: &str, min_tokens: usize) -> bool {
        // Skip trivial whitespace/preamble
        let content = chunk.trim();
        if content.is_empty() {
            return false;
        }

        // Declare useful if:
        // 1. Contains meaningful content (non-whitespace)
        // 2. Contains code fence or newline (structured content)
        // 3. Length exceeds threshold
        content.len() >= min_tokens || content.contains("```") || content.contains('\n')
    }

    fn should_upgrade(chunk: &str) -> bool {
        // Heuristics for quality upgrade:
        // 1. Code blocks
        // 2. Structured responses
        // 3. Tool calls
        chunk.contains("```") ||
        chunk.contains("```") ||
        chunk.contains("function_call") ||
        chunk.contains("tool_call")
    }
}