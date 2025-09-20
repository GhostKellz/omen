# 📡 GhostLLM API Documentation

Complete API reference for GhostLLM Enterprise LLM Proxy.

## 🔗 Base URL

```
Production: https://your-domain.com
Development: http://localhost:8080
```

## 🔐 Authentication

All API requests (except health checks) require authentication via Bearer token:

```http
Authorization: Bearer your-api-key
```

### API Key Types

| Type | Description | Scope |
|------|-------------|-------|
| **User Key** | Standard user access | Chat, models, own usage |
| **Admin Key** | Full administrative access | All endpoints |
| **Service Key** | Service-to-service | Configurable permissions |

## 📋 OpenAI Compatible Endpoints

GhostLLM implements the complete OpenAI API specification for seamless compatibility.

### 🤖 Chat Completions

Create a chat completion response.

**Endpoint:** `POST /v1/chat/completions`

**Request:**
```json
{
  "model": "gpt-4",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant."
    },
    {
      "role": "user",
      "content": "Hello!"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 150,
  "stream": false
}
```

**Response:**
```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 9,
    "total_tokens": 19
  }
}
```

**Streaming Response:**
```http
Content-Type: text/event-stream

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `model` | string | ✅ | Model identifier (e.g., "gpt-4", "claude-3-opus", "llama3:8b") |
| `messages` | array | ✅ | Array of message objects |
| `temperature` | number | ❌ | Sampling temperature (0-2) |
| `max_tokens` | integer | ❌ | Maximum tokens in response |
| `stream` | boolean | ❌ | Enable streaming responses |
| `top_p` | number | ❌ | Nucleus sampling parameter |
| `frequency_penalty` | number | ❌ | Frequency penalty (-2 to 2) |
| `presence_penalty` | number | ❌ | Presence penalty (-2 to 2) |
| `stop` | string/array | ❌ | Stop sequences |

### 📝 Text Completions

Create a text completion response.

**Endpoint:** `POST /v1/completions`

**Request:**
```json
{
  "model": "text-davinci-003",
  "prompt": "The future of AI is",
  "max_tokens": 50,
  "temperature": 0.7
}
```

### 📚 List Models

List available models across all providers.

**Endpoint:** `GET /v1/models`

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "gpt-4",
      "object": "model",
      "created": 1677610602,
      "owned_by": "openai",
      "provider": "openai",
      "context_length": 8192,
      "pricing": {
        "input_per_1k": 0.03,
        "output_per_1k": 0.06
      },
      "capabilities": {
        "vision": false,
        "functions": true,
        "streaming": true
      }
    },
    {
      "id": "claude-3-opus-20240229",
      "object": "model",
      "created": 1677610602,
      "owned_by": "anthropic",
      "provider": "anthropic",
      "context_length": 200000,
      "pricing": {
        "input_per_1k": 0.015,
        "output_per_1k": 0.075
      },
      "capabilities": {
        "vision": true,
        "functions": false,
        "streaming": true
      }
    },
    {
      "id": "llama3:8b",
      "object": "model",
      "created": 1677610602,
      "owned_by": "meta",
      "provider": "ollama",
      "context_length": 4096,
      "pricing": {
        "input_per_1k": 0.0,
        "output_per_1k": 0.0
      },
      "capabilities": {
        "vision": false,
        "functions": false,
        "streaming": true
      }
    }
  ]
}
```

## 🏥 Health & Status Endpoints

### Health Check

**Endpoint:** `GET /health`

**Response:**
```json
{
  "status": "healthy",
  "version": "0.3.0",
  "service": "GhostLLM",
  "timestamp": "1677652288",
  "providers": [
    {
      "id": "openai",
      "name": "OpenAI",
      "healthy": true,
      "models_count": 8
    },
    {
      "id": "anthropic",
      "name": "Anthropic",
      "healthy": true,
      "models_count": 3
    },
    {
      "id": "ollama",
      "name": "Ollama",
      "healthy": true,
      "models_count": 15
    }
  ]
}
```

**Status Codes:**
- `200` - All providers healthy
- `503` - One or more providers unhealthy

### Detailed Status

**Endpoint:** `GET /status`

**Response:**
```json
{
  "status": "running",
  "version": "0.3.0",
  "uptime_seconds": 86400,
  "providers_count": 3,
  "models_count": 26,
  "requests_today": 1542,
  "cache_hit_rate": 0.34
}
```

## 🛡️ Admin API

Administrative endpoints for managing the system.

### Provider Management

#### List Providers

**Endpoint:** `GET /admin/providers`

**Response:**
```json
{
  "providers": [
    {
      "id": "openai",
      "name": "OpenAI",
      "type": "openai",
      "enabled": true,
      "base_url": "https://api.openai.com/v1",
      "models": ["gpt-4", "gpt-3.5-turbo"],
      "health_status": "healthy",
      "last_check": "2024-01-15T10:30:00Z"
    }
  ]
}
```

#### Update Provider

**Endpoint:** `PUT /admin/providers/{provider_id}`

**Request:**
```json
{
  "name": "OpenAI Production",
  "enabled": true,
  "config": {
    "api_key": "sk-...",
    "timeout": 30
  }
}
```

### User Management

#### List Users

**Endpoint:** `GET /admin/users`

**Query Parameters:**
- `page` - Page number (default: 1)
- `limit` - Items per page (default: 20)
- `search` - Search by email/username

**Response:**
```json
{
  "users": [
    {
      "id": "user_123",
      "email": "user@example.com",
      "username": "user123",
      "role": "user",
      "created_at": "2024-01-01T00:00:00Z",
      "last_login": "2024-01-15T10:30:00Z",
      "api_keys_count": 2,
      "usage_this_month": {
        "requests": 150,
        "tokens": 45000,
        "cost": 2.34
      }
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 156,
    "pages": 8
  }
}
```

#### Create User

**Endpoint:** `POST /admin/users`

**Request:**
```json
{
  "email": "new@example.com",
  "username": "newuser",
  "name": "New User",
  "role": "user",
  "budget_limit": 100.0
}
```

### API Key Management

#### List API Keys

**Endpoint:** `GET /admin/keys`

**Response:**
```json
{
  "keys": [
    {
      "id": "key_123",
      "name": "Production Key",
      "user_id": "user_123",
      "permissions": {
        "chat_completions": true,
        "models": ["gpt-4", "claude-3"]
      },
      "rate_limit": 1000,
      "budget_limit": 50.0,
      "current_spend": 12.34,
      "last_used": "2024-01-15T10:30:00Z",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### Create API Key

**Endpoint:** `POST /admin/keys`

**Request:**
```json
{
  "user_id": "user_123",
  "name": "New API Key",
  "permissions": {
    "chat_completions": true,
    "completions": true,
    "models": ["gpt-4"]
  },
  "rate_limit": 500,
  "budget_limit": 25.0,
  "expires_at": "2024-12-31T23:59:59Z"
}
```

**Response:**
```json
{
  "id": "key_456",
  "key": "gk-abc123def456...",
  "name": "New API Key",
  "created_at": "2024-01-15T10:30:00Z"
}
```

## 📊 Analytics & Usage

### Usage Statistics

**Endpoint:** `GET /admin/usage`

**Query Parameters:**
- `start_date` - Start date (ISO 8601)
- `end_date` - End date (ISO 8601)
- `user_id` - Filter by user
- `model` - Filter by model
- `provider` - Filter by provider

**Response:**
```json
{
  "period": {
    "start": "2024-01-01T00:00:00Z",
    "end": "2024-01-15T23:59:59Z"
  },
  "summary": {
    "requests": 15420,
    "tokens": 4521000,
    "cost": 234.56,
    "unique_users": 45
  },
  "by_model": [
    {
      "model": "gpt-4",
      "requests": 5200,
      "tokens": 1800000,
      "cost": 156.30
    }
  ],
  "by_provider": [
    {
      "provider": "openai",
      "requests": 8400,
      "cost": 189.45
    }
  ],
  "daily_breakdown": [
    {
      "date": "2024-01-15",
      "requests": 1200,
      "tokens": 350000,
      "cost": 18.90
    }
  ]
}
```

### Real-time Metrics

**Endpoint:** `GET /metrics`

Prometheus-formatted metrics:

```
# HELP ghostllm_requests_total Total number of requests
# TYPE ghostllm_requests_total counter
ghostllm_requests_total{provider="openai",model="gpt-4",status="success"} 1542

# HELP ghostllm_request_duration_seconds Request duration in seconds
# TYPE ghostllm_request_duration_seconds histogram
ghostllm_request_duration_seconds_bucket{provider="openai",le="0.1"} 45
ghostllm_request_duration_seconds_bucket{provider="openai",le="0.5"} 120
ghostllm_request_duration_seconds_bucket{provider="openai",le="1.0"} 200

# HELP ghostllm_tokens_total Total tokens processed
# TYPE ghostllm_tokens_total counter
ghostllm_tokens_total{provider="openai",model="gpt-4",type="input"} 245000
ghostllm_tokens_total{provider="openai",model="gpt-4",type="output"} 89000
```

## 🔄 WebSocket API

Real-time streaming for chat completions.

**Endpoint:** `ws://localhost:8080/v1/chat/stream`

**Connection:**
```javascript
const ws = new WebSocket('ws://localhost:8080/v1/chat/stream');

// Authentication
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'auth',
    token: 'your-api-key'
  }));
};

// Send chat message
ws.send(JSON.stringify({
  type: 'chat',
  data: {
    model: 'gpt-4',
    messages: [
      {role: 'user', content: 'Hello!'}
    ]
  }
}));

// Receive streaming response
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  if (data.type === 'chunk') {
    console.log(data.content);
  } else if (data.type === 'done') {
    console.log('Response complete');
  }
};
```

## ❌ Error Handling

### Error Response Format

```json
{
  "error": {
    "message": "The model 'invalid-model' does not exist",
    "type": "invalid_request_error",
    "param": "model",
    "code": "model_not_found"
  }
}
```

### Common Error Codes

| Code | Status | Description |
|------|--------|-------------|
| `invalid_api_key` | 401 | Invalid or missing API key |
| `insufficient_quota` | 429 | Rate limit or budget exceeded |
| `model_not_found` | 400 | Requested model not available |
| `invalid_request_error` | 400 | Malformed request |
| `server_error` | 500 | Internal server error |
| `provider_error` | 502 | Upstream provider error |
| `timeout_error` | 504 | Request timeout |

### Rate Limiting

Rate limit headers are included in all responses:

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1677652348
X-RateLimit-Retry-After: 60
```

## 📱 SDKs & Integration

### cURL Examples

**Basic chat completion:**
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

**Streaming chat:**
```bash
curl -N -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Tell me a story"}],
    "stream": true
  }'
```

### Python SDK

```python
import openai

client = openai.OpenAI(
    api_key="your-api-key",
    base_url="http://localhost:8080/v1"
)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "user", "content": "Hello!"}
    ]
)

print(response.choices[0].message.content)
```

### JavaScript SDK

```javascript
import OpenAI from 'openai';

const client = new OpenAI({
  apiKey: 'your-api-key',
  baseURL: 'http://localhost:8080/v1'
});

const response = await client.chat.completions.create({
  model: 'gpt-4',
  messages: [
    {role: 'user', content: 'Hello!'}
  ]
});

console.log(response.choices[0].message.content);
```

## 🌐 OpenWebUI Integration

GhostLLM is fully compatible with OpenWebUI. Configuration:

1. **Open OpenWebUI Settings**
2. **Set API Configuration:**
   - **API Base URL:** `http://localhost:8080/v1`
   - **API Key:** Your GhostLLM API key
3. **Models automatically sync** from GhostLLM

### OpenWebUI Features Supported

- ✅ **Chat Completions** - Full conversation support
- ✅ **Model Switching** - All GhostLLM models available
- ✅ **Streaming** - Real-time response streaming
- ✅ **Vision Models** - Image analysis support
- ✅ **File Upload** - Document processing
- ✅ **Chat History** - Conversation persistence
- ✅ **User Management** - Multi-user support

## 🔧 Advanced Configuration

### Custom Headers

Add provider-specific headers:

```json
{
  "providers": {
    "openai": {
      "headers": {
        "OpenAI-Organization": "org-123456",
        "Custom-Header": "value"
      }
    }
  }
}
```

### Request Transformation

Modify requests before sending to providers:

```json
{
  "transforms": {
    "anthropic": {
      "max_tokens": {
        "default": 1000,
        "max": 4000
      }
    }
  }
}
```

### Caching Configuration

Configure response caching:

```json
{
  "cache": {
    "enabled": true,
    "ttl": 3600,
    "key_template": "{provider}:{model}:{hash}",
    "rules": [
      {
        "pattern": "gpt-4",
        "ttl": 7200
      }
    ]
  }
}
```

---

**For complete API reference and examples, visit the interactive documentation at `http://localhost:8080/docs` when running GhostLLM.**