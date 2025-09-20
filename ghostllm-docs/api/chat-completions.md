# Chat Completions API

The chat completions endpoint is the primary interface for interacting with language models through GhostLLM.

## Endpoint

```
POST /v1/chat/completions
```

## Request Format

```json
{
  "model": "claude-3-sonnet",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "temperature": 0.7,
  "max_tokens": 1000,
  "stream": false
}
```

## Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `model` | string | Yes | Model to use. Use `"auto"` for intelligent routing |
| `messages` | array | Yes | Array of message objects |
| `temperature` | number | No | Sampling temperature (0-2). Default: 1.0 |
| `max_tokens` | integer | No | Maximum tokens to generate |
| `stream` | boolean | No | Enable streaming response. Default: false |
| `stop` | string/array | No | Stop sequences |
| `top_p` | number | No | Nucleus sampling parameter (0-1) |
| `frequency_penalty` | number | No | Frequency penalty (-2 to 2) |
| `presence_penalty` | number | No | Presence penalty (-2 to 2) |
| `functions` | array | No | Available functions for function calling |
| `function_call` | string/object | No | Control function calling behavior |

### GhostLLM-Specific Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `routing` | object | Routing preferences for model selection |
| `session_id` | string | Session ID for conversation continuity |
| `continue_session` | boolean | Continue existing session with different model |
| `provider_hint` | string | Preferred provider (`"ollama"`, `"openai"`, `"anthropic"`) |

## Message Objects

```json
{
  "role": "user|assistant|system|function",
  "content": "Message content",
  "name": "function_name",
  "function_call": {
    "name": "function_name",
    "arguments": "{\"arg\": \"value\"}"
  }
}
```

### Role Types

- **`system`**: Sets the behavior and context for the assistant
- **`user`**: Messages from the user
- **`assistant`**: Messages from the AI assistant
- **`function`**: Results from function calls

## Response Format

### Standard Response

```json
{
  "id": "chatcmpl-7QyqpwdfhqwajicIEznoc6Q47XAyW",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "claude-3-sonnet",
  "provider": "anthropic",
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
    "prompt_tokens": 9,
    "completion_tokens": 12,
    "total_tokens": 21,
    "cost_usd": 0.000063
  },
  "routing": {
    "selected_model": "claude-3-sonnet",
    "selected_provider": "anthropic",
    "reason": "quality_preference",
    "fallback_used": false
  }
}
```

### Streaming Response

When `stream: true` is set, responses are sent as Server-Sent Events:

```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"claude-3-sonnet","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"claude-3-sonnet","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"claude-3-sonnet","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"claude-3-sonnet","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

## Intelligent Routing

Use `"model": "auto"` to enable GhostLLM's intelligent routing:

```json
{
  "model": "auto",
  "messages": [...],
  "routing": {
    "prefer": "quality|speed|cost",
    "max_cost_usd": 0.10,
    "prefer_local": true,
    "fallback": true,
    "exclude_providers": ["expensive_provider"]
  }
}
```

### Routing Options

| Option | Description |
|--------|-------------|
| `prefer` | Optimization strategy: `"quality"`, `"speed"`, or `"cost"` |
| `max_cost_usd` | Maximum cost per request |
| `prefer_local` | Prefer local models (Ollama) when possible |
| `fallback` | Enable fallback to other models if primary fails |
| `exclude_providers` | Array of providers to exclude |

## Function Calling

GhostLLM supports function calling for compatible models:

```json
{
  "model": "gpt-4",
  "messages": [
    {"role": "user", "content": "What's the weather in Boston?"}
  ],
  "functions": [
    {
      "name": "get_weather",
      "description": "Get current weather for a location",
      "parameters": {
        "type": "object",
        "properties": {
          "location": {
            "type": "string",
            "description": "The city and state"
          }
        },
        "required": ["location"]
      }
    }
  ]
}
```

### Function Call Response

```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": null,
        "function_call": {
          "name": "get_weather",
          "arguments": "{\"location\": \"Boston, MA\"}"
        }
      },
      "finish_reason": "function_call"
    }
  ]
}
```

## Examples

### Basic Chat

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-sonnet",
    "messages": [
      {"role": "user", "content": "Explain quantum computing simply"}
    ],
    "max_tokens": 500
  }'
```

### Streaming Response

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "Tell me a story"}
    ],
    "stream": true
  }'
```

### Code Explanation

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "deepseek-coder",
    "messages": [
      {"role": "system", "content": "You are an expert programmer."},
      {"role": "user", "content": "Explain this Rust code:\n\nfn main() {\n    println!(\"Hello, world!\");\n}"}
    ],
    "routing": {
      "prefer": "quality",
      "prefer_local": true
    }
  }'
```

### Multi-turn Conversation

```bash
# First message
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-sonnet",
    "messages": [
      {"role": "user", "content": "I need help with a Python function"}
    ],
    "session_id": "conv-123"
  }'

# Follow-up message
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-sonnet",
    "messages": [
      {"role": "user", "content": "I need help with a Python function"},
      {"role": "assistant", "content": "I'd be happy to help..."},
      {"role": "user", "content": "Can you make it more efficient?"}
    ],
    "session_id": "conv-123"
  }'
```

## Error Handling

### Common Errors

**Model not found:**
```json
{
  "error": {
    "message": "Model 'nonexistent-model' not found",
    "type": "invalid_request_error",
    "param": "model",
    "code": "model_not_found"
  }
}
```

**Rate limit exceeded:**
```json
{
  "error": {
    "message": "Rate limit exceeded",
    "type": "rate_limit_error",
    "code": "rate_limit_exceeded",
    "retry_after": 60
  }
}
```

**GhostWarden consent required:**
```json
{
  "error": {
    "message": "User consent required for this action",
    "type": "consent_required",
    "consent_id": "warden-123",
    "action": {
      "type": "ModelInvoke",
      "model": "gpt-4",
      "estimated_cost": 0.03
    }
  }
}
```

## Best Practices

1. **Use appropriate models**: Choose models based on task complexity
2. **Set reasonable limits**: Use `max_tokens` to control costs
3. **Handle streaming**: Implement proper SSE handling for streaming
4. **Error handling**: Always handle consent and rate limit errors
5. **Session management**: Use session IDs for multi-turn conversations
6. **Cost awareness**: Monitor usage through the routing response

## Model-Specific Considerations

### Claude Models
- Excellent for reasoning and analysis
- Supports large context windows
- Best for complex, nuanced responses

### OpenAI Models
- Great for general tasks
- Good function calling support
- Balanced cost/performance

### Ollama (Local) Models
- No API costs
- Good for development and testing
- May have limited capabilities vs cloud models

### Gemini Models
- Strong multimodal capabilities
- Good for creative tasks
- Competitive performance