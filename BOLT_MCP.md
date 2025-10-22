# Omen Integration with Bolt MCP Gateway

## Overview

Omen serves as the **AI Router and Gateway** for Bolt's MCP infrastructure. When containers make AI tool calls through Glyph MCP servers, Omen provides intelligent, cost-aware, and latency-optimized routing to multiple AI providers.

## Role in Bolt's MCP Architecture

```
Bolt Container → Glyph MCP → Bolt MCP Gateway → Omen Router → AI Providers
                                                      ↓
                                    ┌─────────────────┴─────────────────┐
                                    │                                   │
                            Claude/OpenAI/Gemini              Local Ollama
                            (Cloud Providers)                 (4090/3070)
```

**Omen's Responsibilities:**
- Smart AI provider routing (cost, latency, intent-based)
- Multi-provider support (Claude, GPT, Gemini, Ollama, etc.)
- Usage tracking and quota enforcement
- Rate limiting and budget controls
- Provider health monitoring and failover

## Why Omen for Bolt?

### Intelligent Routing
- **Intent-aware:** Automatically classifies requests (code, reasoning, vision, etc.)
- **Cost-optimized:** Routes to cheapest provider meeting quality requirements
- **Latency-first:** Prefers local Ollama for fast, simple tasks
- **Adaptive:** Learns from usage patterns and optimizes over time

### Multi-Provider Support
- **Cloud:** Anthropic Claude, OpenAI GPT, Google Gemini, xAI Grok, Azure OpenAI, AWS Bedrock
- **Local:** Ollama (multi-instance support for 4090/3070 clusters)
- **Unified API:** OpenAI-compatible `/v1/*` endpoints
- **Provider abstraction:** Clients don't need provider-specific code

### Usage Controls
- **Per-container quotas:** Limit AI spend by service
- **Rate limiting:** Prevent abuse and runaway costs
- **Budget tracking:** Real-time cost monitoring
- **Soft/hard caps:** Warnings before hard limits

### Production-Ready
- **High availability:** Health checks, failover, retry logic
- **Observability:** Comprehensive metrics and tracing
- **SSO integration:** Google/GitHub/Microsoft OIDC
- **Audit logging:** Track all AI requests for compliance

## Integration Architecture

### Request Flow

```
1. Container makes AI request
   ↓
2. Glyph MCP server receives tool call
   ↓
3. Bolt MCP Gateway forwards to Omen
   ↓
4. Omen analyzes request:
   - Intent classification (code/reasoning/vision)
   - Cost estimation
   - Latency requirements
   - Current quotas and budgets
   ↓
5. Omen selects provider:
   - Local Ollama (fast, free)
   - Or cloud provider (quality, capabilities)
   ↓
6. Request sent to provider
   ↓
7. Response streamed back through chain
   ↓
8. Usage tracked, metrics recorded
```

### Configuration Mapping

Bolt's Boltfile Omen configuration:

```toml
[services.app.omen]
enabled = true
router_strategy = "cost-optimized"    # cost-optimized, latency-first, balanced
prefer_local = true                   # Prefer Ollama when possible
budget_limit = "10.00"                # USD per day
allowed_providers = ["ollama", "anthropic", "openai"]

[services.app.omen.quotas]
max_requests_per_hour = 1000
max_tokens_per_day = 500000
max_cost_per_day = "10.00"

[services.app.omen.routing]
intents.code = "ollama"               # Code tasks → local
intents.reasoning = "anthropic"       # Reasoning → Claude
intents.vision = "openai"             # Vision → GPT-4V
```

↓ Translates to Omen config ↓

```toml
# omen.toml
[routing]
default_strategy = "cost-optimized"
prefer_local = true

[routing.intent_rules]
code = { provider = "ollama", model = "deepseek-coder:6.7b" }
reasoning = { provider = "anthropic", model = "claude-sonnet-4" }
vision = { provider = "openai", model = "gpt-4-vision-preview" }

[quotas.container_app]
max_requests_per_hour = 1000
max_tokens_per_day = 500000
budget_daily = 10.00

[providers.ollama]
endpoints = ["http://localhost:11434", "http://ollama-3070:11434"]
cost_per_token = 0.0
latency_target_ms = 100

[providers.anthropic]
api_key = "env:OMEN_ANTHROPIC_API_KEY"
cost_per_1k_tokens = 0.003

[providers.openai]
api_key = "env:OMEN_OPENAI_API_KEY"
cost_per_1k_tokens = 0.001
```

## Deployment Modes

### 1. Shared Omen Instance (Recommended)

Single Omen instance serves all Bolt containers:

```yaml
# docker-compose.yml
services:
  omen:
    image: ghcr.io/ghostkellz/omen:latest
    environment:
      OMEN_BIND: "0.0.0.0:8080"
      OMEN_REDIS_URL: "redis://redis:6379"
      OMEN_ANTHROPIC_API_KEY: "${ANTHROPIC_API_KEY}"
      OMEN_OPENAI_API_KEY: "${OPENAI_API_KEY}"
      OMEN_OLLAMA_ENDPOINTS: "http://ollama:11434"
    ports:
      - "8080:8080"

  redis:
    image: redis:7-alpine

  ollama:
    image: ollama/ollama:latest
    volumes:
      - ollama-data:/root/.ollama
```

```toml
# Boltfile.toml
[omen]
endpoint = "http://localhost:8080"
api_key = "env:BOLT_OMEN_KEY"

[services.app.omen]
enabled = true
```

**Advantages:**
- Centralized quota management
- Efficient provider connection pooling
- Simplified configuration

### 2. Per-Container Omen (Advanced)

Each container gets dedicated Omen instance:

```toml
[services.app-omen]
image = "omen:latest"
environment = [
  "OMEN_PROVIDERS=ollama",
  "OMEN_OLLAMA_ENDPOINT=http://ollama:11434"
]
network_mode = "container:app"
```

**Advantages:**
- Complete isolation
- Per-service provider configuration
- No shared quota contention

**Use Cases:**
- Multi-tenant environments
- High-security workloads
- Different provider sets per service

### 3. Hybrid (Local + Shared)

Local Ollama per container, shared cloud routing:

```toml
[services.app]
image = "my-app:latest"

[services.app-ollama]
image = "ollama/ollama:latest"
network_mode = "container:app"

[services.app.omen]
enabled = true
endpoint = "http://shared-omen:8080"
prefer_local = true
local_endpoint = "http://localhost:11434"
```

## Routing Strategies

### Cost-Optimized

Minimizes spend while meeting quality requirements:

```toml
[services.app.omen]
router_strategy = "cost-optimized"
quality_threshold = 0.8        # Minimum acceptable quality (0-1)
```

**Logic:**
1. Try free local Ollama first
2. If quality insufficient or model unavailable, use cheapest cloud provider
3. Track cumulative cost

**Use Cases:**
- High-volume, simple requests
- Development/testing environments
- Budget-constrained deployments

### Latency-First

Minimizes response time:

```toml
[services.app.omen]
router_strategy = "latency-first"
latency_target_ms = 100
```

**Logic:**
1. Prefer local Ollama (lowest latency)
2. If unavailable, use cloud provider with best latency history
3. Adapt based on real-time measurements

**Use Cases:**
- Interactive applications
- Gaming containers (real-time AI assist)
- User-facing features

### Balanced

Balances cost, latency, and quality:

```toml
[services.app.omen]
router_strategy = "balanced"
cost_weight = 0.3
latency_weight = 0.4
quality_weight = 0.3
```

**Logic:**
1. Score each provider: `score = cost_weight * cost + latency_weight * latency + quality_weight * quality`
2. Select provider with best score
3. Re-evaluate periodically

**Use Cases:**
- Production workloads
- General-purpose AI features
- Most deployments (default)

### Intent-Based

Route by request intent:

```toml
[services.app.omen.routing]
intents.code = "ollama"                # Local for code
intents.reasoning = "anthropic"        # Claude for reasoning
intents.vision = "openai"              # GPT-4V for images
intents.math = "openai"                # GPT-4 for math
intents.agent = "anthropic"            # Claude for agents
```

**Logic:**
1. Classify request intent from messages/metadata
2. Route to configured provider for that intent
3. Fall back to default strategy if intent unclear

**Use Cases:**
- Specialized workloads
- Optimizing for provider strengths
- Complex multi-intent applications

## Usage Controls

### Container-Level Quotas

```toml
[services.app.omen.quotas]
max_requests_per_hour = 1000
max_requests_per_day = 10000
max_tokens_per_day = 500000
max_cost_per_day = "10.00"            # USD
```

**Enforcement:**
- Hard limits return `429 Too Many Requests`
- Soft limits (90% threshold) return warnings
- Redis-backed for distributed tracking

### Budget Management

```toml
[services.app.omen.budget]
daily = "10.00"
weekly = "50.00"
monthly = "150.00"

# Alerts
alert_at_percent = 80
alert_webhook = "https://slack.webhook.url"

# Actions when exceeded
on_exceed = "deny"                    # deny, warn, or ignore
```

### Rate Limiting

```toml
[services.app.omen.rate_limit]
requests_per_second = 10
burst = 20
tokens_per_minute = 10000
```

## Provider Configuration

### Local Ollama (Multi-Instance)

```toml
[providers.ollama]
endpoints = [
  "http://ollama-4090:11434",         # Primary (high-performance)
  "http://ollama-3070:11434",         # Secondary
]
models = [
  "deepseek-coder:6.7b",
  "llama3.1:8b-instruct",
  "qwen2.5:7b-instruct"
]
load_balancing = "round-robin"        # round-robin, least-loaded, random
health_check_interval = "30s"
cost_per_token = 0.0
```

### Cloud Providers

```toml
[providers.anthropic]
api_key = "env:OMEN_ANTHROPIC_API_KEY"
models = ["claude-sonnet-4", "claude-opus-4"]
cost_per_1k_input_tokens = 0.003
cost_per_1k_output_tokens = 0.015
rate_limit = 50                       # req/s
timeout = "60s"

[providers.openai]
api_key = "env:OMEN_OPENAI_API_KEY"
models = ["gpt-4-turbo", "gpt-4o"]
cost_per_1k_input_tokens = 0.01
cost_per_1k_output_tokens = 0.03
rate_limit = 100
timeout = "60s"

[providers.gemini]
api_key = "env:OMEN_GEMINI_API_KEY"
models = ["gemini-pro", "gemini-ultra"]
cost_per_1k_tokens = 0.00025
rate_limit = 60
```

## API Integration

### OpenAI-Compatible Endpoint

Bolt MCP Gateway connects to Omen via OpenAI-compatible API:

```rust
// In Bolt MCP Gateway
use reqwest::Client;

let client = Client::new();
let response = client
    .post("http://omen:8080/v1/chat/completions")
    .header("Authorization", format!("Bearer {}", api_key))
    .json(&serde_json::json!({
        "model": "auto",           // Omen auto-selects
        "messages": messages,
        "stream": true,
        "metadata": {
            "container_id": container_id,
            "intent": "code"
        }
    }))
    .send()
    .await?;
```

### Metadata for Routing

```json
{
  "model": "auto",
  "messages": [...],
  "metadata": {
    "container_id": "bolt-dev-env",
    "intent": "code",
    "priority": "low-latency",
    "budget_remaining": "5.50",
    "prefer_local": true
  }
}
```

Omen uses metadata to make routing decisions.

## Observability

### Metrics

Omen exposes Prometheus metrics:

```
# Requests
omen_requests_total{container, provider, model, status}
omen_request_duration_seconds{container, provider, model}

# Routing
omen_routing_decisions_total{strategy, provider, reason}
omen_provider_selections_total{provider, intent}

# Cost
omen_cost_usd_total{container, provider}
omen_tokens_total{container, provider, type="input|output"}

# Quotas
omen_quota_usage{container, quota_type, limit}
omen_quota_exceeded_total{container, quota_type}

# Provider health
omen_provider_health{provider, endpoint}
omen_provider_latency_seconds{provider, percentile}
```

### Tracing

Omen integrates with OpenTelemetry:

```rust
use opentelemetry::trace::Tracer;

// In Bolt MCP Gateway
let span = tracer.start("mcp_ai_tool_call");
span.set_attribute("container_id", container_id);
span.set_attribute("tool", tool_name);

// Request to Omen propagates trace context
let response = client.post(omen_url)
    .header("traceparent", span.trace_id())
    .send()
    .await?;
```

**Trace visualization:** See full request path from container → Glyph → Gateway → Omen → Provider

### Logging

```bash
# Container-level logs
bolt logs app --omen

# Omen logs
docker logs omen

# Filter by container
omen logs --container app --level debug
```

## Gaming Container Example

Gaming container with AI assistant using Omen:

```toml
[services.game]
image = "bolt://steam:latest"

[services.game.gaming.gpu]
runtime = "nvbind"
isolation_level = "exclusive"

[services.game.mcp]
enabled = true
tools = ["game_state", "mod_loader", "performance_advisor"]

[services.game.omen]
enabled = true
router_strategy = "latency-first"       # Ultra-low latency for gaming
allowed_providers = ["ollama"]          # Local only (no network latency)
max_requests_per_hour = 5000            # High volume for real-time assist

[services.game.omen.routing]
intents.game_assist = "ollama"
intents.mod_recommend = "ollama"
intents.performance = "ollama"
```

**Use Case:** In-game AI assistant provides:
- Real-time strategy suggestions
- Mod recommendations
- Performance optimization advice

All routed to local Ollama for <100ms latency.

## Development Container Example

Dev container with AI coding assistant:

```toml
[services.dev]
image = "bolt://dev-env:latest"

[services.dev.mcp]
enabled = true
tools = ["filesystem", "shell", "git", "build"]

[services.dev.omen]
enabled = true
router_strategy = "balanced"
prefer_local = true
budget_limit = "5.00"                   # $5/day for dev work

[services.dev.omen.routing]
intents.code = "ollama"                 # Simple code → local
intents.reasoning = "anthropic"         # Complex reasoning → Claude
intents.refactor = "anthropic"          # Refactoring → Claude
intents.explain = "ollama"              # Explanations → local
```

**Use Case:** AI coding assistant (Claude Code, Cursor, Zeke) connects to container:
- Simple tasks (explain, simple edits) → free local Ollama
- Complex tasks (architecture, refactoring) → Claude Sonnet
- Automatic cost optimization

## Cost Analysis

### Example Workload

**Scenario:** Development container, 8 hours/day usage

| Task Type    | Volume/Day | Provider   | Cost/Day |
|--------------|------------|------------|----------|
| Code explain | 100 calls  | Ollama     | $0.00    |
| Simple edits | 50 calls   | Ollama     | $0.00    |
| Refactoring  | 10 calls   | Claude     | $0.50    |
| Architecture | 5 calls    | Claude     | $1.00    |
| **Total**    | 165 calls  | Mixed      | **$1.50**|

**Without Omen (Claude only):** 165 × $0.03 = $4.95/day

**Savings:** 70% cost reduction via intelligent routing

### Gaming Workload

**Scenario:** Gaming container, real-time AI assist

| Task Type        | Volume/Hour | Provider | Cost/Hour |
|------------------|-------------|----------|-----------|
| Strategy tips    | 50 calls    | Ollama   | $0.00     |
| Mod suggestions  | 10 calls    | Ollama   | $0.00     |
| Perf analysis    | 5 calls     | Ollama   | $0.00     |
| **Total**        | 65 calls    | Local    | **$0.00** |

**Without Omen (cloud only):** Not feasible (latency + cost)

**Benefit:** Real-time AI assistance becomes viable

## Security

### API Key Management

```toml
# Global Omen API keys
[omen.auth]
api_keys = [
  { key = "env:BOLT_OMEN_MASTER_KEY", scope = "admin" },
  { key = "env:BOLT_OMEN_USER_KEY", scope = "user" }
]

# Per-container keys
[services.app.omen]
api_key = "env:APP_OMEN_KEY"
```

### Provider Key Isolation

Omen stores provider API keys securely:
- Never exposed to containers
- Encrypted at rest (Redis)
- Rotated regularly
- Audit logging on access

### Request Validation

```rust
// Omen validates all requests
if request.metadata.container_id != authenticated_container {
    return Err(AuthError::ContainerMismatch);
}

if request.cost_estimate > container_budget_remaining {
    return Err(QuotaError::BudgetExceeded);
}
```

## Health Checks

Omen provides health endpoints for Bolt:

```bash
# Basic health (liveness)
curl http://omen:8080/health
# → 200 OK { "status": "healthy", "providers": [...] }

# Readiness (at least one provider available)
curl http://omen:8080/ready
# → 200 OK (ready) or 503 Service Unavailable (not ready)

# Provider status
curl http://omen:8080/providers
# → { "ollama": "healthy", "anthropic": "healthy", "openai": "degraded" }
```

Bolt uses these for:
- Container health checks
- Gateway routing decisions
- Automatic failover

## Bolt Integration Checklist

- [ ] Add Omen service to Bolt deployment (Docker Compose/Kubernetes)
- [ ] Implement Omen client in `bolt/src/mcp/omen_client.rs`
- [ ] Parse Omen config from Boltfile
- [ ] Wire up MCP Gateway → Omen requests
- [ ] Implement container quota tracking
- [ ] Add Omen metrics to Bolt monitoring
- [ ] Configure provider API keys
- [ ] Set up health checks
- [ ] Write integration tests
- [ ] Document usage patterns

## Resources

- **Omen Documentation:** `docs/`
- **Omen API Reference:** `API_ROUTING.md`
- **Provider Setup:** `QUICKSTART.md`
- **Bolt Integration RFC:** `../bolt/BOLT_MCP.md`

## Support

For Omen-specific issues:
- GitHub: https://github.com/ghostkellz/omen/issues
- Discord: https://discord.gg/ghoststack

---

**Status:** Production-Ready (Omen v0.1.0)
**Bolt Integration:** Planning Phase
**Maintainer:** @ghostkellz
