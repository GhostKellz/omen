# 🌐 OpenWebUI Integration Guide

Complete guide for integrating GhostLLM with OpenWebUI for a seamless chat experience.

## 🎯 Overview

GhostLLM provides **100% OpenAI API compatibility**, making it a perfect drop-in replacement for OpenAI's API in OpenWebUI. This allows you to:

- **Use multiple LLM providers** through a single interface
- **Centralize API key management** and usage tracking
- **Add enterprise features** like rate limiting and cost controls
- **Self-host everything** for complete privacy and control

## 🚀 Quick Setup

### 1. Start GhostLLM

```bash
# Using Docker (recommended)
cd ghostllm
docker-compose up -d

# Or using the binary
./target/release/ghostllm-proxy serve
```

**Verify GhostLLM is running:**
```bash
curl http://localhost:8080/health
```

### 2. Configure OpenWebUI

**In OpenWebUI Settings:**

1. **Open Settings** → **Connections**
2. **Set OpenAI API Configuration:**
   - **API Base URL:** `http://localhost:8080/v1`
   - **API Key:** Your GhostLLM API key (or `dev-key` for development)
3. **Save Configuration**

**Models will automatically sync** from GhostLLM!

### 3. Test Integration

1. **Check Models** - Available models should appear in OpenWebUI
2. **Start Chat** - Create a new conversation
3. **Select Model** - Choose from any GhostLLM provider (OpenAI, Anthropic, Ollama, etc.)
4. **Send Message** - Test the chat functionality

## ⚙️ Configuration

### GhostLLM Configuration

**Environment Variables (.env):**
```bash
# API Configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# Authentication (set to false for testing)
ENABLE_AUTH=false  # For development
# ENABLE_AUTH=true  # For production

# API Keys for providers
OPENAI_API_KEY=sk-your-openai-key
ANTHROPIC_API_KEY=sk-ant-your-anthropic-key

# Database (if using authentication)
DATABASE_URL=postgresql://ghostllm:password@localhost:5432/ghostllm
```

### OpenWebUI Configuration

**docker-compose.yml for OpenWebUI:**
```yaml
version: '3.8'

services:
  open-webui:
    image: ghcr.io/open-webui/open-webui:main
    container_name: open-webui
    volumes:
      - open-webui:/app/backend/data
    ports:
      - "3000:8080"
    environment:
      - OPENAI_API_BASE_URL=http://ghostllm:8080/v1
      - OPENAI_API_KEY=your-ghostllm-api-key
    extra_hosts:
      - "host.docker.internal:host-gateway"
    restart: unless-stopped

  ghostllm:
    image: ghostllm:latest
    container_name: ghostllm
    ports:
      - "8080:8080"
    env_file:
      - .env
    restart: unless-stopped

volumes:
  open-webui:
```

## 🔐 Authentication Setup

### Development Mode (No Auth)

For testing and development:

```bash
# In GhostLLM .env
ENABLE_AUTH=false

# In OpenWebUI - use any API key
OPENAI_API_KEY=dev-key
```

### Production Mode (With Auth)

**1. Create GhostLLM API Key:**
```bash
# Create admin user first
cargo run --bin ghostllm-cli user create \
  --email admin@yourcompany.com \
  --role admin

# Create API key
cargo run --bin ghostllm-cli apikey create \
  --user admin@yourcompany.com \
  --name "OpenWebUI Key" \
  --permissions chat,models
```

**2. Configure OpenWebUI:**
```bash
# Use the generated API key
OPENAI_API_KEY=gk-your-generated-api-key
```

## 🎛️ Advanced Configuration

### Per-User API Keys

**Create user-specific API keys:**

```bash
# Create user
ghostllm-cli user create \
  --email user@company.com \
  --role user \
  --budget-limit 100.00

# Create API key with specific permissions
ghostllm-cli apikey create \
  --user user@company.com \
  --name "User Chat Key" \
  --permissions chat \
  --rate-limit 1000 \
  --budget-limit 50.00
```

### Model Restrictions

**Restrict access to specific models:**

```bash
# Create API key with model restrictions
ghostllm-cli apikey create \
  --user user@company.com \
  --name "Limited Access" \
  --models "gpt-3.5-turbo,llama3:8b" \
  --rate-limit 500
```

### Budget Controls

**Set spending limits:**

```json
{
  "apiKey": "gk-...",
  "permissions": {
    "chat_completions": true,
    "models": ["gpt-3.5-turbo"]
  },
  "budgetLimit": 50.00,
  "rateLimitPerHour": 1000
}
```

## 🔄 Model Management

### Available Models

GhostLLM automatically exposes models from all configured providers:

**OpenAI Models:**
- `gpt-4` - GPT-4 (128k context)
- `gpt-3.5-turbo` - GPT-3.5 Turbo
- `gpt-4-vision-preview` - GPT-4 with vision

**Anthropic Models:**
- `claude-3-opus-20240229` - Claude 3 Opus
- `claude-3-sonnet-20240229` - Claude 3 Sonnet
- `claude-3-haiku-20240307` - Claude 3 Haiku

**Ollama Models (if configured):**
- `llama3:8b` - Llama 3 8B
- `mistral:latest` - Mistral 7B
- `codellama:7b` - Code Llama 7B

### Model Selection in OpenWebUI

1. **Automatic Detection** - Models appear automatically
2. **Model Switching** - Switch between providers seamlessly
3. **Model Information** - Context length and pricing info available
4. **Custom Names** - Configure friendly names for models

### Model Configuration

**In GhostLLM config:**
```toml
[providers.openai]
enabled = true
models = ["gpt-4", "gpt-3.5-turbo"]

[providers.anthropic]
enabled = true
models = ["claude-3-opus-20240229"]

[providers.ollama]
enabled = true
base_url = "http://localhost:11434"
# Models auto-detected
```

## 📊 Usage Tracking

### Real-time Analytics

GhostLLM tracks all usage automatically:

- **Token Usage** - Input and output tokens per request
- **Cost Tracking** - Real-time cost calculation
- **User Activity** - Per-user and per-API-key metrics
- **Model Popularity** - Most used models and providers

### Analytics Dashboard

**Access via GhostLLM Admin:**
```bash
# Open admin dashboard
open http://localhost:8080/admin

# Or use CLI
ghostllm-cli analytics --user user@company.com --period 30d
```

### OpenWebUI Integration

**Usage appears in OpenWebUI:**
- Chat history preserved
- User sessions tracked
- Model usage visible
- Cost information available (if enabled)

## 🎨 Customization

### Custom Model Names

**Map provider models to friendly names:**
```toml
[model_aliases]
"gpt-4" = "GPT-4 (OpenAI)"
"claude-3-opus-20240229" = "Claude 3 Opus (Anthropic)"
"llama3:8b" = "Llama 3 8B (Local)"
```

### Provider Routing

**Route models to different providers:**
```toml
[routing]
# Route gpt-4 requests to Anthropic for cost savings
"gpt-4" = "claude-3-sonnet-20240229"

# Load balancing
"gpt-3.5-turbo" = ["openai", "azure-openai"]
```

### Custom Headers

**Add provider-specific headers:**
```toml
[providers.openai.headers]
"OpenAI-Organization" = "org-your-org-id"
"Custom-Header" = "value"
```

## 🔧 Troubleshooting

### Common Issues

**1. Models not appearing in OpenWebUI**

```bash
# Check GhostLLM health
curl http://localhost:8080/health

# Check models endpoint
curl http://localhost:8080/v1/models

# Verify OpenWebUI can reach GhostLLM
curl -H "Authorization: Bearer your-api-key" \
     http://localhost:8080/v1/models
```

**2. Authentication errors**

```bash
# Check API key format
echo "API Key: $OPENAI_API_KEY"

# Test API key
curl -H "Authorization: Bearer $OPENAI_API_KEY" \
     http://localhost:8080/v1/models

# Check GhostLLM logs
docker-compose logs ghostllm
```

**3. Connection refused**

```bash
# Check if GhostLLM is running
curl http://localhost:8080/health

# Check Docker network
docker network ls
docker network inspect ghostllm_default

# Check port mapping
docker-compose ps
```

**4. Rate limiting issues**

```bash
# Check rate limit headers
curl -I -H "Authorization: Bearer your-api-key" \
       http://localhost:8080/v1/models

# Increase rate limits
ghostllm-cli apikey update your-key-id --rate-limit 5000
```

### Debug Mode

**Enable debug logging:**
```bash
# In GhostLLM
RUST_LOG=debug docker-compose up ghostllm

# Check specific modules
RUST_LOG=ghostllm_proxy::handlers=debug docker-compose up
```

### Health Checks

**Comprehensive health check:**
```bash
# GhostLLM health
curl http://localhost:8080/health

# Provider health
curl http://localhost:8080/admin/providers

# OpenWebUI connectivity test
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-3.5-turbo", "messages": [{"role": "user", "content": "test"}], "max_tokens": 5}'
```

## 📈 Performance Optimization

### Caching

**Enable response caching:**
```toml
[cache]
enabled = true
ttl = 3600  # 1 hour
provider_ttl = 300  # 5 minutes for provider responses
```

### Connection Pooling

**Optimize provider connections:**
```toml
[providers.openai]
max_connections = 50
timeout = 30
retry_attempts = 3
```

### Rate Limiting

**Configure appropriate limits:**
```toml
[rate_limiting]
default_per_minute = 1000
default_per_hour = 10000
burst_size = 100
```

## 🚀 Production Deployment

### Docker Compose Production

**docker-compose.prod.yml:**
```yaml
version: '3.8'

services:
  ghostllm:
    image: ghostllm:latest
    restart: unless-stopped
    environment:
      - RUST_LOG=info
      - ENABLE_AUTH=true
    env_file:
      - .env.prod
    volumes:
      - ./ssl:/etc/ssl
    networks:
      - app-network

  open-webui:
    image: ghcr.io/open-webui/open-webui:main
    restart: unless-stopped
    environment:
      - OPENAI_API_BASE_URL=http://ghostllm:8080/v1
      - OPENAI_API_KEY=${GHOSTLLM_API_KEY}
    volumes:
      - open-webui-data:/app/backend/data
    networks:
      - app-network

  nginx:
    image: nginx:alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - ghostllm
      - open-webui
    networks:
      - app-network

networks:
  app-network:

volumes:
  open-webui-data:
```

### Nginx Configuration

**nginx.conf:**
```nginx
upstream ghostllm {
    server ghostllm:8080;
}

upstream openwebui {
    server open-webui:8080;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    # SSL configuration
    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;

    # OpenWebUI frontend
    location / {
        proxy_pass http://openwebui;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # GhostLLM API
    location /api/ {
        proxy_pass http://ghostllm/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## 📚 Additional Resources

### Example Configurations

- [**Docker Setup**](examples/docker/docker-compose.openwebui.yml)
- [**Kubernetes**](examples/kubernetes/openwebui-ghostllm.yaml)
- [**Nginx Config**](examples/nginx/openwebui.conf)

### Video Tutorials

- **Setup Walkthrough** - Step-by-step setup guide
- **Configuration Deep Dive** - Advanced configuration options
- **Troubleshooting** - Common issues and solutions

### Community

- **GitHub Discussions** - Questions and community support
- **Discord** - Real-time help and community chat
- **Examples Repository** - Community-contributed configurations

---

**🌐 Enjoy seamless LLM access with OpenWebUI + GhostLLM!**

*The perfect combination of powerful UI and enterprise-grade backend.*