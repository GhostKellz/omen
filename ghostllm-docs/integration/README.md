# Integration Guides

This section provides comprehensive guides for integrating GhostLLM with popular development tools and frameworks.

## Quick Integration Overview

GhostLLM provides an OpenAI-compatible API, making it a drop-in replacement for most existing integrations. Simply change the base URL from `https://api.openai.com/v1` to your GhostLLM instance.

## Available Integrations

### Editor Plugins

- **[Neovim Plugins](neovim.md)** - Integrate with Claude.nvim, ChatGPT.nvim, and other AI plugins
- **[VS Code Extensions](vscode.md)** - Use with Copilot, Claude, and custom extensions
- **[Emacs Integration](emacs.md)** - GPTel and other Emacs AI packages

### CLI Tools

- **[Zeke CLI](zeke.md)** - High-performance Rust-native AI coding assistant
- **[Shell Tools](shell.md)** - Integration with command-line AI tools
- **[Git Hooks](git.md)** - AI-powered commit messages and code review

### Development Frameworks

- **[Python Applications](python.md)** - OpenAI Python client and frameworks
- **[Node.js Applications](nodejs.md)** - OpenAI Node.js client and libraries
- **[Rust Applications](rust.md)** - async-openai and custom implementations
- **[Go Applications](go.md)** - OpenAI Go client integration

### Specialized Tools

- **[Continue.dev](continue.md)** - VS Code/JetBrains AI coding assistant
- **[Aider](aider.md)** - AI pair programming tool
- **[Cursor](cursor.md)** - AI-first code editor
- **[GitHub Copilot Chat](copilot.md)** - Copilot with GhostLLM backend

## General Integration Pattern

Most integrations follow this pattern:

### 1. Update Base URL
```bash
# Before
export OPENAI_BASE_URL="https://api.openai.com/v1"

# After
export OPENAI_BASE_URL="http://localhost:8080/v1"
```

### 2. Configure Authentication
```bash
# For local models (Ollama)
export OPENAI_API_KEY=""

# For authenticated access
export OPENAI_API_KEY="your-ghostllm-token"
```

### 3. Test Connection
```bash
curl -X POST $OPENAI_BASE_URL/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

## GhostLLM-Specific Features

### Intelligent Model Routing

Most tools allow model specification. Use these special values:

- `"auto"` - Let GhostLLM choose the best model
- `"local"` - Prefer local Ollama models
- `"cloud"` - Use cloud providers only
- `"fast"` - Optimize for speed
- `"quality"` - Optimize for quality
- `"cheap"` - Optimize for cost

### GhostWarden Integration

When using tools that make file system or command changes, configure consent handling:

```json
{
  "ghostwarden": {
    "auto_approve_read": true,
    "auto_approve_write": false,
    "project_scope": "repo:current"
  }
}
```

### Session Persistence

For multi-turn conversations, use session IDs:

```json
{
  "session_id": "your-session-id",
  "continue_session": true
}
```

## Common Configuration Examples

### Development Environment
```toml
# ~/.config/ghostllm/integration.toml
[development]
default_model = "auto"
prefer_local = true
auto_approve_read = true
auto_approve_write = true  # Safe for dev
cost_limit_daily = 5.00

[models.routing]
code_completion = "deepseek-coder"
code_explanation = "claude-3-sonnet"
general_chat = "llama3:8b"
```

### Production Environment
```toml
[production]
default_model = "claude-3-sonnet"
prefer_local = false
auto_approve_read = false
auto_approve_write = false  # Strict consent
cost_limit_daily = 50.00
require_mfa = true

[models.routing]
code_review = "gpt-4"
documentation = "claude-3-sonnet"
testing = "claude-3-haiku"  # Faster/cheaper
```

## Integration Checklist

When integrating a new tool with GhostLLM:

- [ ] **API Compatibility**: Verify the tool uses OpenAI-compatible API
- [ ] **Base URL Configuration**: Update endpoint to GhostLLM
- [ ] **Authentication**: Configure API key or remove for local-only
- [ ] **Model Selection**: Choose appropriate model or use `"auto"`
- [ ] **Error Handling**: Handle GhostWarden consent prompts
- [ ] **Streaming Support**: Enable streaming if the tool supports it
- [ ] **Cost Monitoring**: Set up usage tracking and limits
- [ ] **Testing**: Verify functionality with both local and cloud models

## Troubleshooting

### Common Issues

1. **Connection Refused**
   - Ensure GhostLLM proxy is running
   - Check firewall and network settings
   - Verify port configuration

2. **Authentication Errors**
   - Check API key configuration
   - Verify GhostLLM auth settings
   - Test with curl first

3. **Model Not Found**
   - List available models: `GET /v1/models`
   - Check provider configuration
   - Verify model names in GhostLLM config

4. **Consent Timeouts**
   - Implement proper consent handling
   - Configure appropriate timeouts
   - Use auto-approval for development

### Debug Mode

Most tools support debug/verbose mode. Enable it to see:
- API requests and responses
- Model selection decisions
- GhostWarden interactions
- Error details

### Health Checks

Before integration, verify GhostLLM health:

```bash
# Service health
curl http://localhost:8080/health

# Available models
curl http://localhost:8080/v1/models

# Provider status
curl http://localhost:8080/admin/providers
```

## Contributing Integration Guides

Have a tool integration working with GhostLLM? We welcome contributions:

1. Fork the repository
2. Add your integration guide to `docs/integration/`
3. Include configuration examples and troubleshooting
4. Submit a pull request

## Support

- **Integration Issues**: [GitHub Issues](https://github.com/ghostkellz/ghostllm/issues)
- **General Questions**: [GitHub Discussions](https://github.com/ghostkellz/ghostllm/discussions)
- **Community**: [Discord Server](https://discord.gg/ghostllm)