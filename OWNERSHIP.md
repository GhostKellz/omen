Glyph (Rust) → MCP brain & guardrails: protocol, sessions, transports, schemas, consent/audit, observability.

Rune (Zig) → Hot path engine: ultra-fast file/workspace ops + low-latency provider calls; exports C ABI; great for CLIs/editors.

Omen (Rust) → OpenAI-compatible AI gateway: auth/quotas/routing/streaming/tool-use across providers (OpenAI, Anthropic, Azure, Ollama, Bedrock…), with “model=auto” + budgets.

Mental model

Think of Omen as the API front door for LLMs; Glyph as the protocol hub for tools/resources (MCP); Rune as the turbocharged worker you call for anything performance-sensitive.

Clients (Zeke CLI, zeke.nvim, Jarvis, GhostFlow, services)
           │
           ▼
        OMEN  (OpenAI-compatible API, auth/quotas/routing, SSE/WS)
           │        ▲
           │        │  provider adapters (OpenAI/Anthropic/Azure/Ollama/…)
           ▼        │
       Providers  (cloud + local GPUs via Ollama)

MCP world (tools/resources):
  Glyph (MCP server/client, schemas, consent/audit, metrics)
     ↕
  Rune (fast file/workspace/search/selection, provider helpers via FFI)

Who owns what (crisp split)
Concern	Glyph	Rune	Omen
Primary job	MCP protocol + governance	Fast local operations & provider calls from apps	OpenAI-compat API + routing/auth/quotas
I/O shape	JSON-RPC 2.0 (MCP)	Direct calls/FFI; thin MCP client/server	/v1/* (OpenAI-style), SSE/WS
Security	Consent gates, policy hooks, audit signing	Enforce local guards; defer policy to Glyph	OIDC/API keys, org/workspace RBAC, rate limits, spend caps
Observability	tracing + Prometheus + request IDs	lightweight counters; exports to Glyph/Omen	per-request cost/latency, budgets, audit logs
Schemas	JSON-Schema/OpenAPI for tools/resources	Compile-time bindings; optional runtime validate	Normalizes tool/function calling across providers
Where it runs	Services/daemons & tool hosts	CLIs/editors/daemons that need speed	Edge/API gateway in front of all models
Two clean deployment patterns
A) Omen under GhostLLM (recommended early)

Clients (Zeke.nvim/Zeke/Jarvis/GhostFlow) → GhostLLM → Omen → providers.

Pros: one place (GhostLLM) to evolve enterprise features; Omen focuses on OpenAI-compat & routing.

Use when: GhostLLM already centralizes policy and you want drop-in /v1 for tools and editors.

B) GhostLLM under Omen

Clients → Omen → (providers + GhostLLM as a provider) → models.

Pros: everything speaks OpenAI-compat; “model=auto” + quotas live in Omen; GhostLLM can specialize (e.g., advanced caching/RAG).

Use when: you want a single URL for all clients and Omen’s admin/budget view.

Either way, Glyph stays your MCP authority, Rune stays your fast worker. Omen doesn’t replace either; it replaces adhoc provider glue with a real gateway.

Typical flows

Editor request (code-assist):
Zeke.nvim → Omen (model=auto, tags: intent=code) → chooses local Ollama-4090 → streams tokens.
If a tool is needed (e.g., “read_file”), the client calls Glyph (MCP) → Glyph enforces consent/audit → calls Rune FFI for the fast file read → returns result to the editor.

Agentic task (Jarvis):
Jarvis planner → Omen for reasoning → receives tool_calls → forwards tool execution to Glyph MCP tools → some tools implemented via Rune for speed.

Avoiding overlap (guardrails)

Do not put quotas/SSO/OpenAI-compat into Glyph or Rune → that’s Omen.

Do not re-implement MCP consent/audit in Omen or Rune → that’s Glyph.

Do not move fast file/workspace primitives into Glyph or Omen → keep in Rune, expose via FFI/MCP.

Minimal wiring choices

Clients to Omen: point OPENAI_API_BASE (or OMEN_URL) at Omen; send /v1/chat/completions with model=auto.

Glyph ↔ Rune: Glyph tools import rune-ffi for hot paths; Glyph remains the MCP front for tools.

GhostLLM ↔ Omen: set either GHOSTLLM_UPSTREAM=<OMEN_URL> (A) or point Omen to GhostLLM as a “provider” (B).

When to reach for which

Need one URL for all models, usage accounting, budgets, SSO, streaming? → Omen.

Need protocol-correct tools/resources, consent/audit logs, schemas, multi-transport? → Glyph.

Need sub-ms selections, huge workspace scans, SIMD search, low-alloc pipelines? → Rune.
