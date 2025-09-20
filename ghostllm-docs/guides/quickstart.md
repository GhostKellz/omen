# Quick Start Guide

Get GhostLLM up and running in minutes with this step-by-step guide.

## Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Git**: For cloning the repository
- **Ollama** (optional): For local models

## 1. Installation

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/ghostkellz/ghostllm
cd ghostllm

# Build the project
cargo build --release
```

### Verify Installation

```bash
# Check the binary
./target/release/ghostllm-proxy --help
```

## 2. Basic Configuration

Create a minimal configuration file:

```bash
mkdir -p config
cat > config/app.toml << EOF
[server]
host = "127.0.0.1"
port = 8080

[providers.ollama]
enabled = true
base_url = "http://localhost:11434"
provider_type = "ollama"

[security]
enable_auth = false  # For local development
EOF
```

## 3. Start the Proxy

```bash
# Start in development mode
./target/release/ghostllm-proxy serve --dev

# Or with custom config
./target/release/ghostllm-proxy serve --config config/app.toml
```

You should see output like:
```
🚀 GhostLLM proxy server starting on 127.0.0.1:8080
✅ Server listening on http://127.0.0.1:8080
🔍 Health check: http://127.0.0.1:8080/health
📚 API docs: http://127.0.0.1:8080/docs
```

## 4. Test the API

### Health Check

```bash
curl http://localhost:8080/health
```

Expected response:
```json
{
  "status": "healthy",
  "version": "0.3.0",
  "service": "GhostLLM",
  "providers": [...]
}
```

### List Models

```bash
curl http://localhost:8080/v1/models
```

### First Chat Request

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "Hello! Can you help me with coding?"}
    ]
  }'
```

## 5. Add Cloud Providers (Optional)

### OpenAI

```toml
# Add to config/app.toml
[providers.openai]
enabled = true
provider_type = "openai"
base_url = "https://api.openai.com/v1"
api_key = "your-openai-api-key"
```

### Anthropic Claude

```toml
[providers.anthropic]
enabled = true
provider_type = "anthropic"
base_url = "https://api.anthropic.com/v1"
api_key = "your-anthropic-api-key"
```

### Restart the Proxy

```bash
# Stop with Ctrl+C and restart
./target/release/ghostllm-proxy serve --config config/app.toml
```

## 6. Install Ollama (Local Models)

### Install Ollama

```bash
# macOS/Linux
curl -fsSL https://ollama.ai/install.sh | sh

# Windows
# Download from https://ollama.ai/download
```

### Pull Models

```bash
# Fast, general purpose
ollama pull llama3:8b

# Coding specialist
ollama pull deepseek-coder:6.7b

# Small and fast
ollama pull phi3:mini
```

### Verify Ollama Integration

```bash
curl http://localhost:8080/v1/models | grep -i llama
```

## 7. Test Different Models

### Use Specific Model

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama3:8b",
    "messages": [
      {"role": "user", "content": "Explain Python lists"}
    ]
  }'
```

### Intelligent Routing

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "Write a Rust function to sort a vector"}
    ],
    "routing": {
      "prefer": "quality",
      "prefer_local": true
    }
  }'
```

## 8. Enable GhostWarden (Optional)

Create a basic security policy:

```bash
cat > config/warden.toml << EOF
default_action = "prompt"
consent_timeout_secs = 30

[capabilities]
"model.invoke" = "allow"
"fs.read" = "allow"
"fs.write" = "prompt"

[scopes]
"repo:ghostllm".overrides = { "fs.write" = "allow" }
EOF
```

Update main config:

```toml
# Add to config/app.toml
[security]
enable_ghostwarden = true
warden_config = "config/warden.toml"
```

## 9. Test Streaming

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "model": "llama3:8b",
    "messages": [
      {"role": "user", "content": "Tell me a short story about AI"}
    ],
    "stream": true
  }'
```

## 10. Next Steps

### Integrate with Your Tools

- **Neovim**: See [Neovim Integration Guide](../GHOSTLLM_NVIM_PLUGIN.md)
- **Zeke CLI**: See [Zeke Integration Guide](../GHOSTLLM_ZEKE_CLI.md)
- **VS Code**: Configure extensions to use `http://localhost:8080/v1`

### Monitor Usage

```bash
# Check provider status
curl http://localhost:8080/admin/providers

# View usage stats (if enabled)
curl http://localhost:8080/admin/usage
```

### Configure Advanced Features

- **Rate Limiting**: Set per-user/per-model limits
- **Cost Tracking**: Monitor API usage costs
- **Custom Routing**: Define intelligent routing rules
- **Authentication**: Set up API keys and user management

## Common Issues

### Port Already in Use

```bash
# Check what's using port 8080
lsof -i :8080

# Use different port
./target/release/ghostllm-proxy serve --port 8081
```

### Ollama Connection Failed

```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# Start Ollama if needed
ollama serve
```

### Models Not Loading

```bash
# Check Ollama models
ollama list

# Pull a model if none exist
ollama pull llama3:8b
```

### Permission Denied

```bash
# Make binary executable
chmod +x ./target/release/ghostllm-proxy

# Check file permissions
ls -la ./target/release/ghostllm-proxy
```

## Configuration Examples

### Development Setup

```toml
[server]
host = "127.0.0.1"
port = 8080

[providers.ollama]
enabled = true
provider_type = "ollama"
base_url = "http://localhost:11434"

[logging]
level = "debug"
enable_request_logging = true

[security]
enable_auth = false
enable_ghostwarden = false
```

### Production Setup

```toml
[server]
host = "0.0.0.0"
port = 8080

[providers.openai]
enabled = true
provider_type = "openai"
api_key = "${OPENAI_API_KEY}"

[providers.anthropic]
enabled = true
provider_type = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"

[security]
enable_auth = true
enable_ghostwarden = true
api_keys = ["your-secure-api-key"]

[rate_limits]
requests_per_minute = 60
cost_per_day_usd = 10.0
```

## Help and Support

- **Documentation**: [Full Documentation](../README.md)
- **API Reference**: [API Docs](../api/README.md)
- **Issues**: [GitHub Issues](https://github.com/ghostkellz/ghostllm/issues)
- **Community**: [Discord](https://discord.gg/ghostllm)

## What's Next?

1. **Configure your favorite editor** with GhostLLM integration
2. **Set up cloud providers** for more model options
3. **Enable monitoring** and cost tracking
4. **Explore advanced routing** and fallback strategies
5. **Join the community** and share your experience!