# GhostLLM Documentation

Welcome to the GhostLLM documentation. GhostLLM is a unified LLM proxy and routing engine built in Rust that provides secure, cost-effective access to multiple AI providers.

## Quick Links

- [API Reference](api/) - Complete API documentation
- [Integration Guides](integration/) - How to integrate with your tools
- [User Guides](guides/) - Step-by-step tutorials
- [Examples](examples/) - Working code examples

## What is GhostLLM?

GhostLLM is an enterprise-grade LLM proxy that:

- **Unifies Multiple Providers**: Claude, OpenAI, Ollama, Gemini, and more through one API
- **Intelligent Routing**: Automatically choose the best model for each task
- **Security First**: GhostWarden consent system for safe AI interactions
- **Cost Optimization**: Track usage, set limits, prefer local models
- **High Performance**: Rust-native with async streaming support

## Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Your App      │───▶│   GhostLLM      │───▶│   AI Providers  │
│                 │    │   Proxy         │    │                 │
│ • Claude.nvim   │    │ • Routing       │    │ • Claude        │
│ • ChatGPT.nvim  │    │ • GhostWarden   │    │ • OpenAI        │
│ • Zeke CLI      │    │ • Analytics     │    │ • Ollama        │
│ • Custom Apps   │    │ • Caching       │    │ • Gemini        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Key Features

### 🚀 **Multi-Provider Support**
- OpenAI (GPT-4, GPT-3.5)
- Anthropic (Claude 3 Family)
- Local Ollama (Llama, DeepSeek, Qwen)
- Google Gemini
- Extensible provider system

### 🛡️ **GhostWarden Security**
- Consent-based AI interactions
- Project-scoped permissions
- Rate limiting and quotas
- Audit logging

### 🎯 **Intelligent Routing**
- Automatic model selection
- Cost-aware routing
- Latency optimization
- Fallback strategies

### 💰 **Cost Management**
- Real-time usage tracking
- Budget limits and alerts
- Prefer local models
- Detailed analytics

## Getting Started

### 1. Installation

```bash
git clone https://github.com/ghostkellz/ghostllm
cd ghostllm
cargo build --release
```

### 2. Configuration

```toml
# config/app.toml
[server]
host = "127.0.0.1"
port = 8080

[providers.ollama]
enabled = true
base_url = "http://localhost:11434"

[providers.openai]
enabled = true
api_key = "your-openai-key"
```

### 3. Start the Proxy

```bash
./target/release/ghostllm-proxy serve
```

### 4. Test the API

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

## Documentation Sections

### [API Reference](api/)
Complete OpenAI-compatible API documentation:
- [Chat Completions](api/chat-completions.md)
- [Models](api/models.md)
- [Streaming](api/streaming.md)
- [Authentication](api/auth.md)

### [Integration Guides](integration/)
How to integrate GhostLLM with popular tools:
- [Neovim Plugins](integration/neovim.md)
- [VS Code Extensions](integration/vscode.md)
- [Zeke CLI](integration/zeke.md)
- [Custom Applications](integration/custom.md)

### [User Guides](guides/)
Step-by-step tutorials:
- [Quick Start](guides/quickstart.md)
- [Configuration](guides/configuration.md)
- [GhostWarden Setup](guides/ghostwarden.md)
- [Provider Setup](guides/providers.md)
- [Monitoring](guides/monitoring.md)

### [Examples](examples/)
Working code examples:
- [Rust Client](examples/rust-client/)
- [Python Client](examples/python-client/)
- [JavaScript Client](examples/js-client/)
- [Neovim Integration](examples/nvim/)

## FAQ

**Q: Is GhostLLM compatible with OpenAI clients?**
A: Yes! GhostLLM implements the OpenAI API specification, so any OpenAI-compatible client works.

**Q: Can I use local models only?**
A: Absolutely. Configure only Ollama providers for a fully local setup.

**Q: How does GhostWarden consent work?**
A: GhostWarden intercepts requests and prompts for user approval based on configurable policies.

**Q: Does GhostLLM store my conversations?**
A: Only if you enable logging. By default, GhostLLM is stateless and doesn't persist conversations.

## Contributing

We welcome contributions! See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## Support

- **Issues**: [GitHub Issues](https://github.com/ghostkellz/ghostllm/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ghostkellz/ghostllm/discussions)
- **Discord**: [Join our Discord](https://discord.gg/ghostllm)

## License

MIT License. See [LICENSE](../LICENSE) for details.