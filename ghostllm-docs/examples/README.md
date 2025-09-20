# GhostLLM Examples

This directory contains working examples of GhostLLM integration with various languages, frameworks, and tools.

## Quick Examples

### Basic API Usage

```bash
# Simple chat completion
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Streaming response
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "model": "claude-3-sonnet",
    "messages": [{"role": "user", "content": "Tell me a story"}],
    "stream": true
  }'
```

## Language Examples

### Python

**Basic Client**
```python
from openai import OpenAI

# Initialize client
client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key=""  # Empty for local, or your key
)

# Simple completion
response = client.chat.completions.create(
    model="auto",
    messages=[
        {"role": "user", "content": "Explain Python decorators"}
    ]
)

print(response.choices[0].message.content)
```

**Streaming Example**
```python
response = client.chat.completions.create(
    model="claude-3-sonnet",
    messages=[{"role": "user", "content": "Write a Python function"}],
    stream=True
)

for chunk in response:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

### JavaScript/Node.js

**Basic Client**
```javascript
import OpenAI from 'openai';

const openai = new OpenAI({
  baseURL: 'http://localhost:8080/v1',
  apiKey: '' // Empty for local
});

async function chat() {
  const response = await openai.chat.completions.create({
    model: 'auto',
    messages: [
      { role: 'user', content: 'Explain async/await in JavaScript' }
    ]
  });

  console.log(response.choices[0].message.content);
}

chat();
```

**Streaming Example**
```javascript
async function streamChat() {
  const stream = await openai.chat.completions.create({
    model: 'llama3:8b',
    messages: [{ role: 'user', content: 'Tell me about Node.js' }],
    stream: true
  });

  for await (const chunk of stream) {
    process.stdout.write(chunk.choices[0]?.delta?.content || '');
  }
}
```

### Rust

**Basic Client**
```rust
use async_openai::{Client, types::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::with_config(
        async_openai::config::OpenAIConfig::new()
            .with_api_base("http://localhost:8080/v1")
            .with_api_key("")
    );

    let request = CreateChatCompletionRequestArgs::default()
        .model("auto")
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a helpful Rust expert.")
                .build()?.into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content("Explain Rust ownership")
                .build()?.into(),
        ])
        .build()?;

    let response = client.chat().completions().create(request).await?;

    println!("{}", response.choices[0].message.content.as_ref().unwrap());
    Ok(())
}
```

### Go

**Basic Client**
```go
package main

import (
    "context"
    "fmt"
    "github.com/sashabaranov/go-openai"
)

func main() {
    config := openai.DefaultConfig("")
    config.BaseURL = "http://localhost:8080/v1"
    client := openai.NewClientWithConfig(config)

    resp, err := client.CreateChatCompletion(
        context.Background(),
        openai.ChatCompletionRequest{
            Model: "auto",
            Messages: []openai.ChatCompletionMessage{
                {
                    Role:    openai.ChatMessageRoleUser,
                    Content: "Explain Go goroutines",
                },
            },
        },
    )

    if err != nil {
        fmt.Printf("Error: %v\n", err)
        return
    }

    fmt.Println(resp.Choices[0].Message.Content)
}
```

## Tool Integration Examples

### Neovim Plugin Integration

**ChatGPT.nvim Configuration**
```lua
require("chatgpt").setup({
  api_host_cmd = "echo http://localhost:8080",
  api_key_cmd = "echo ''",
  openai_params = {
    model = "auto",
    frequency_penalty = 0,
    presence_penalty = 0,
    max_tokens = 4000,
    temperature = 0.2,
    top_p = 0.1,
    n = 1,
  },
})
```

**Claude.nvim Configuration**
```lua
require('claude').setup({
  base_url = "http://localhost:8080/v1",
  api_key = "",
  model = "claude-3-sonnet",

  -- GhostLLM-specific features
  features = {
    model_switching = true,
    cost_awareness = true,
    local_fallback = true,
  }
})
```

### Zeke CLI Integration

**Configuration**
```toml
# ~/.config/zeke/zeke.toml
[api]
base_url = "http://localhost:8080/v1"
api_key = ""

[models]
default = "auto"
code_completion = "deepseek-coder"
chat = "claude-3-sonnet"

[ghostllm]
enable_routing = true
enable_consent = true
session_persistence = true
```

**Usage Examples**
```bash
# Interactive chat
zeke chat

# Code explanation
zeke explain src/main.rs

# Generate tests
zeke test --file src/lib.rs

# Use specific model
zeke chat --model claude-3-sonnet
```

## Advanced Examples

### Function Calling

```python
import json
from openai import OpenAI

client = OpenAI(base_url="http://localhost:8080/v1", api_key="")

def get_weather(location):
    # Mock weather function
    return f"The weather in {location} is sunny, 72°F"

response = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "user", "content": "What's the weather in Boston?"}
    ],
    functions=[
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
)

if response.choices[0].message.function_call:
    function_call = response.choices[0].message.function_call
    args = json.loads(function_call.arguments)
    result = get_weather(args["location"])
    print(result)
```

### Intelligent Routing

```python
# Optimize for cost
response = client.chat.completions.create(
    model="auto",
    messages=[{"role": "user", "content": "Simple greeting"}],
    extra_body={
        "routing": {
            "prefer": "cost",
            "max_cost_usd": 0.01,
            "prefer_local": True
        }
    }
)

# Optimize for quality
response = client.chat.completions.create(
    model="auto",
    messages=[{"role": "user", "content": "Complex analysis needed"}],
    extra_body={
        "routing": {
            "prefer": "quality",
            "exclude_providers": ["local"]
        }
    }
)
```

### Batch Processing

```python
import asyncio
from openai import AsyncOpenAI

async def process_files(files):
    client = AsyncOpenAI(base_url="http://localhost:8080/v1", api_key="")

    tasks = []
    for file_path in files:
        with open(file_path, 'r') as f:
            content = f.read()

        task = client.chat.completions.create(
            model="deepseek-coder",
            messages=[
                {"role": "system", "content": "Analyze this code for bugs"},
                {"role": "user", "content": f"File: {file_path}\n\n{content}"}
            ]
        )
        tasks.append(task)

    results = await asyncio.gather(*tasks)
    return results

# Usage
files = ["src/main.rs", "src/lib.rs", "src/utils.rs"]
results = asyncio.run(process_files(files))
```

## Error Handling Examples

### Handling GhostWarden Consent

```python
def handle_consent_error(error_data):
    """Handle GhostWarden consent requirement"""
    if error_data.get("type") == "consent_required":
        consent_id = error_data["consent_id"]
        action = error_data["action"]

        print(f"Consent required for: {action}")
        user_input = input("Allow? (y/n): ")

        if user_input.lower() == 'y':
            # Send consent approval
            import requests
            requests.post(
                "http://localhost:8080/admin/consent",
                json={
                    "consent_id": consent_id,
                    "decision": "allow_once"
                }
            )
            return True
    return False

try:
    response = client.chat.completions.create(...)
except Exception as e:
    if "consent_required" in str(e):
        # Parse error and handle consent
        pass
```

### Rate Limit Handling

```python
import time
from openai import RateLimitError

def chat_with_retry(client, **kwargs):
    max_retries = 3
    for attempt in range(max_retries):
        try:
            return client.chat.completions.create(**kwargs)
        except RateLimitError as e:
            if attempt < max_retries - 1:
                wait_time = 2 ** attempt  # Exponential backoff
                print(f"Rate limited, waiting {wait_time}s...")
                time.sleep(wait_time)
            else:
                raise
```

## Configuration Examples

### Development Environment

```python
# development_client.py
from openai import OpenAI

def create_dev_client():
    return OpenAI(
        base_url="http://localhost:8080/v1",
        api_key="",  # No auth for dev
        timeout=30.0,
        max_retries=2,
    )

# Default to local models for development
def dev_chat(content, model="auto"):
    client = create_dev_client()
    return client.chat.completions.create(
        model=model,
        messages=[{"role": "user", "content": content}],
        extra_body={
            "routing": {
                "prefer_local": True,
                "max_cost_usd": 0.01
            }
        }
    )
```

### Production Environment

```python
# production_client.py
import os
from openai import OpenAI

def create_prod_client():
    return OpenAI(
        base_url=os.getenv("GHOSTLLM_URL", "https://ghostllm.company.com/v1"),
        api_key=os.getenv("GHOSTLLM_API_KEY"),
        timeout=60.0,
        max_retries=3,
    )

def prod_chat(content, model="claude-3-sonnet"):
    client = create_prod_client()
    return client.chat.completions.create(
        model=model,
        messages=[{"role": "user", "content": content}],
        extra_body={
            "routing": {
                "prefer": "quality",
                "max_cost_usd": 0.50
            }
        }
    )
```

## Running the Examples

Each example directory contains:
- `README.md` - Specific setup instructions
- Source code files
- Configuration examples
- Test scripts

### Python Examples
```bash
cd examples/python-client
pip install -r requirements.txt
python basic_chat.py
```

### Node.js Examples
```bash
cd examples/nodejs-client
npm install
node basic_chat.js
```

### Rust Examples
```bash
cd examples/rust-client
cargo run --example basic_chat
```

### Tool Integration Examples
```bash
cd examples/nvim-integration
# Follow setup instructions in README.md
```

## Contributing Examples

We welcome new examples! Please:

1. Create a new directory under the appropriate category
2. Include a complete, working example
3. Add clear documentation and setup instructions
4. Test with both local and cloud models
5. Handle errors gracefully

## Support

- **Example Issues**: [GitHub Issues](https://github.com/ghostkellz/ghostllm/issues)
- **Feature Requests**: [GitHub Discussions](https://github.com/ghostkellz/ghostllm/discussions)
- **Community**: [Discord Server](https://discord.gg/ghostllm)