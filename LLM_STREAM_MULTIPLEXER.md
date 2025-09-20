Put the LLM Stream Multiplexer in OMEN.
OMEN is your unified API + routing layer; the multiplexer is a routing strategy. GhostLLM can either call OMEN (and inherit it) or keep a simpler router. This avoids duplicating complex logic in two places.

Use HTTP/3 over QUIC internally (and optionally to clients).
QUIC’s stream multiplexing + no HOL blocking is perfect for “race-to-first-token” and speculative fan-out. You can still expose SSE/WS to clients while using H3/QUIC upstream.

QUIC library? 
quinn and quiche are solid crates
Also I have a home brewed one: https://github.com/ghostkellz/gquic

Here’s a tight design you can build right into OMEN.

Where it lives

OMEN → owns “routing strategies”: single, race, speculate_k, parallel_merge.
GhostLLM → either forwards to OMEN (preferred) or uses single.

API shape (OpenAI-compat with extensions)

Client sets strategy via tags/headers; you keep OpenAI semantics:

POST /v1/chat/completions
{
  "model": "auto",
  "messages": [...],
  "stream": true,
  "omen": {
    "strategy": "race",            // single | race | speculate_k | parallel_merge
    "k": 2,                        // for speculate_k/parallel_merge
    "providers": ["anthropic","ollama","openai"], // optional allowlist
    "budget_usd": 0.15,            // cap per request
    "max_latency_ms": 3500,        // SLO; cancel losers after first token or deadline
    "stickiness": "session"        // none | turn | session
  }
}


Streaming response remains standard; OMEN handles the fan-out/swap invisibly.

Multiplexer strategies

race (recommended default)

Fan out to N candidates (e.g., Ollama-4090 + Claude + GPT).

The first provider to deliver a useful token “wins”; cancel others.

Great for latency; cheapest if you pick k=2 (local + 1 cloud).

speculate_k

Start fast local (Ollama) immediately; start cloud with a jittered delay (e.g., 120–250ms).

If cloud overtakes with better quality (heuristic or explicit upgrade), switch streams mid-flight and cancel local.

Minimizes perceived latency while allowing quality upgrade.

parallel_merge

Run N in parallel and merge best spans (rare; researchy).

Useful for evals or high-stakes summarization; costly.

QUIC/H3 placement

Client ↔ OMEN: keep SSE or WS for broad compatibility. Add HTTP/3 as an option later.

OMEN ↔ Providers:

Ollama: HTTP/1.1; keep local; low RTT.

OpenAI/Anthropic/Grok/Gemini: mostly H2 today. Wrap with H3 when providers allow (future).

OMEN internal fan-out:

Use QUIC (quinn) + h3 inside your network if you have microservices or sidecars (e.g., OMEN↔Wraith, OMEN↔GhostLLM). This is where QUIC shines: multiplexed streams, no HOL blocking, nice cancellation semantics.

Core mechanics (concise)

Bidirectional controller

One task per provider (Tokio).

A select! loop consumes whichever stream yields first non-trivial token.

On win: send cancel to losers; drain briefly for cleanup; close.

First-useful logic

Ignore trivial whitespace/preamble tokens.

Declare winner when token length ≥ N or contains code fence/newline.

Backpressure + timeouts

max_latency_ms per request (budget).

per_provider_deadline = base_slo + jitter.

If nobody responds, degrade to single best provider.

Cost caps

Track prompt+completion tokens per provider; abort others if budget_usd would be exceeded by continuing parallel runs.

Consistency

Keep session stickiness after a win (e.g., rest of the conversation prefers winner unless user asked strategy: race every time).

Mid-stream upgrade (speculate_k)

If slow provider produces higher-quality signal (e.g., longer coherent span, tool-call with high confidence), emit a stream_event: upgrade(provider, model) to the client (optional) or silently swap and continue.

Tool-call arbitration

If different providers emit different tool calls, prefer the winner.

Optionally gate with OMEN Rules (“only allowed tools”), or ask client to choose if tool names diverge.

Rust sketch (ultra-condensed)

Crates: tokio, futures, reqwest (or per-provider clients), quinn + h3 (internal), tracing.

enum Strategy { Single, Race, SpeculateK(usize), ParallelMerge(usize) }

async fn mux_stream(req: ChatReq, candidates: Vec<Provider>, strat: Strategy) -> StreamOut {
    match strat {
        Strategy::Single => run_single(req, candidates[0]).await,
        Strategy::Race => race(req, candidates).await,
        Strategy::SpeculateK(k) => speculate(req, candidates, k).await,
        Strategy::ParallelMerge(k) => merge(req, candidates, k).await,
    }
}

async fn race(req: ChatReq, mut cands: Vec<Provider>) -> StreamOut {
    let (tx, rx) = mpsc::channel(32);
    let cancels = CancellationTokens::new();

    for p in cands.iter() {
        let reqp = req.clone(); let txp = tx.clone(); let c = cancels.child();
        tokio::spawn(async move { stream_provider(reqp, p, c, txp).await; });
    }

    // first useful token wins
    let mut winner: Option<ProviderId> = None;
    while let Some(ev) = rx.recv().await {
        match ev {
            Ev::Token{provider, chunk} if useful(&chunk) && winner.is_none() => {
                winner = Some(provider.clone());
                cancels.cancel_all_except(&provider);
                yield_to_client(ev);
            },
            Ev::Token{provider, ..} if Some(provider) == winner => yield_to_client(ev),
            Ev::Done{provider} if Some(provider) == winner => break,
            _ => {} // ignore losers
        }
    }
    finalize()
}

Where does Wraith fit?

If you build Wraith (AI-aware reverse proxy), it can:

Terminate TLS/H3, multiplex to OMEN over QUIC with mTLS.

Do edge rate-limits and pass JWT claims to OMEN.

Optionally implement race-to-first at the edge for identical upstreams (e.g., two OMEN nodes) for HA.

But the multi-provider race logic belongs in OMEN, where provider adapters live and costs are known.

Defaults I’d ship

strategy = "race" with 2 candidates: ollama-4090 + one cloud (Claude/OpenAI based on quotas).

max_latency_ms = 3000 (tune per endpoint).

budget_usd = 0.05 per request for race (cheap and fast).

Stickiness = session to avoid flip-flopping in a conversation.

Enable speculate_k=2 for editor integrations (Zeke) to get “instant typing” from local and silent swap to cloud when needed.

TL;DR

Implement the LLM Stream Multiplexer inside OMEN as routing strategies (race, speculate_k, parallel_merge).

Use QUIC/H3 internally for multiplexing and clean cancellation; expose standard SSE/WS to clients.

Keep Wraith for edge TLS/H3 + auth + quotas; keep GhostLLM simpler or make it an OMEN client.

You get lower latency (race), better quality (speculative upgrade), and cost control (budget caps) without client changes.
