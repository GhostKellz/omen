# GhostLLM API Reference

GhostLLM provides an OpenAI-compatible API that unifies access to multiple LLM providers. All endpoints follow OpenAI specifications, making it a drop-in replacement for existing applications.

## Base URL

```
http://localhost:8080/v1
```

## Authentication

GhostLLM supports multiple authentication methods:

- **Anonymous**: For local-only models (Ollama)
- **API Key**: For authenticated sessions
- **Session Token**: For web applications

```bash
# Anonymous request
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json"

# With API key
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key"
```

## Endpoints

### Core API

- [`POST /v1/chat/completions`](chat-completions.md) - Chat completions with streaming support
- [`GET /v1/models`](models.md) - List available models
- [`POST /v1/embeddings`](embeddings.md) - Generate embeddings (if supported)

### Health & Status

- [`GET /health`](health.md) - Service health check
- [`GET /status`](status.md) - Detailed status information

### Admin API

- [`GET /admin/providers`](admin.md#providers) - List provider status
- [`POST /admin/consent`](admin.md#consent) - Handle GhostWarden consent
- [`GET /admin/usage`](admin.md#usage) - Usage statistics

## Request/Response Format

All requests and responses use JSON format with OpenAI-compatible schemas.

### Standard Response

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "claude-3-sonnet",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello! How can I help you today?"
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 9,
    "completion_tokens": 12,
    "total_tokens": 21
  }
}
```

### Error Response

```json
{
  "error": {
    "message": "Model not found",
    "type": "invalid_request_error",
    "param": "model",
    "code": "model_not_found"
  }
}
```

## Special Features

### Intelligent Routing

Use `"model": "auto"` to let GhostLLM choose the optimal model:

```json
{
  "model": "auto",
  "messages": [{"role": "user", "content": "Explain quantum computing"}],
  "routing": {
    "prefer": "quality",
    "fallback": true,
    "max_cost": 0.10
  }
}
```

### GhostWarden Integration

When consent is required, you'll receive:

```json
{
  "error": {
    "message": "User consent required for this action",
    "type": "consent_required",
    "consent_id": "uuid-here",
    "action": {
      "type": "ModelInvoke",
      "model": "gpt-4",
      "estimated_cost": 0.03
    }
  }
}
```

### Model Switching

Switch models mid-conversation by changing the model parameter:

```json
{
  "model": "claude-3-opus",
  "messages": [...],
  "continue_session": true,
  "session_id": "your-session-id"
}
```

## Rate Limits

GhostLLM implements intelligent rate limiting:

- **Per-user limits**: Configurable per API key
- **Per-model limits**: Different limits for different models
- **Cost-based limits**: Daily/monthly spending limits
- **Provider limits**: Respect upstream provider limits

Rate limit headers:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1677652400
X-RateLimit-Cost: 0.002
```

## Streaming

GhostLLM supports Server-Sent Events (SSE) streaming:

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "model": "claude-3-sonnet",
    "messages": [{"role": "user", "content": "Tell me a story"}],
    "stream": true
  }'
```

## Error Codes

| Code | Description |
|------|-------------|
| `400` | Bad Request - Invalid parameters |
| `401` | Unauthorized - Missing or invalid API key |
| `403` | Forbidden - GhostWarden policy violation |
| `404` | Not Found - Model or endpoint not found |
| `429` | Too Many Requests - Rate limit exceeded |
| `500` | Internal Server Error - Server error |
| `502` | Bad Gateway - Provider error |
| `503` | Service Unavailable - Temporary unavailability |

## OpenAI Compatibility

GhostLLM is designed to be a drop-in replacement for OpenAI's API. Supported features:

✅ Chat Completions
✅ Streaming
✅ Function Calling
✅ System Messages
✅ Temperature/Top-p
✅ Max Tokens
✅ Stop Sequences
❌ Fine-tuning (not applicable)
❌ Image Generation (provider-dependent)
⚠️ Embeddings (provider-dependent)

## Provider-Specific Features

Different providers support different features:

| Feature | OpenAI | Claude | Ollama | Gemini |
|---------|--------|--------|--------|---------|
| Streaming | ✅ | ✅ | ✅ | ✅ |
| Functions | ✅ | ✅ | ⚠️ | ✅ |
| Vision | ✅ | ✅ | ⚠️ | ✅ |
| System | ✅ | ✅ | ✅ | ✅ |

## SDKs and Clients

Since GhostLLM implements the OpenAI API, you can use any OpenAI-compatible client:

### Python
```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="your-key-or-empty"
)
```

### Node.js
```javascript
import OpenAI from 'openai';

const openai = new OpenAI({
  baseURL: 'http://localhost:8080/v1',
  apiKey: 'your-key-or-empty'
});
```

### Rust
```rust
use async_openai::{Client, config::OpenAIConfig};

let config = OpenAIConfig::new()
    .with_api_base("http://localhost:8080/v1")
    .with_api_key("your-key-or-empty");
let client = Client::with_config(config);
```

## Next Steps

- [Chat Completions API](chat-completions.md) - Detailed chat API documentation
- [Models API](models.md) - Available models and capabilities
- [Streaming Guide](streaming.md) - Real-time streaming implementation
- [Authentication](auth.md) - Authentication and security