# 👨‍💻 GhostLLM Development Guide

Complete development setup and contribution guide for GhostLLM.

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.70+ with Cargo
- **Docker** and Docker Compose
- **PostgreSQL** 13+ (or use Docker)
- **Redis** 6+ (or use Docker)
- **Git** for version control

### One-Command Setup

```bash
# Clone and setup everything
git clone https://github.com/yourusername/ghostllm
cd ghostllm
chmod +x setup.sh
./setup.sh --skip-test

# Start development
make dev
```

## 🛠️ Development Environment Setup

### 1. Install Rust Toolchain

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install required targets and tools
rustup component add clippy rustfmt
rustup target add wasm32-unknown-unknown  # For web frontend

# Install additional tools
cargo install cargo-watch  # For hot reload
cargo install cargo-expand # For macro debugging
cargo install sqlx-cli     # For database management
```

### 2. Clone and Configure

```bash
# Clone repository
git clone https://github.com/yourusername/ghostllm
cd ghostllm

# Create environment file
cp .env.example .env

# Edit .env with your settings
nano .env
```

**Essential `.env` Configuration:**
```bash
# Development database (use Docker or local)
DATABASE_URL=postgresql://ghostllm:ghostllm123@localhost:5432/ghostllm
REDIS_URL=redis://localhost:6379

# Development secrets (use simple values for dev)
JWT_SECRET=dev-jwt-secret-key
ADMIN_API_KEY=dev-admin-key

# API Keys (optional for basic testing)
OPENAI_API_KEY=sk-your-openai-key  # Optional
ANTHROPIC_API_KEY=sk-ant-...       # Optional

# Development settings
RUST_LOG=debug
GHOSTLLM_ENV=development
```

### 3. Start Development Services

**Option A: Using Docker (Recommended)**
```bash
# Start database and Redis only
docker-compose up -d postgres redis

# Verify services
docker-compose ps
```

**Option B: Local Installation**
```bash
# Install PostgreSQL
sudo apt-get install postgresql postgresql-contrib
sudo -u postgres createuser --createdb ghostllm
sudo -u postgres psql -c "ALTER USER ghostllm PASSWORD 'ghostllm123';"
createdb -U ghostllm ghostllm

# Install Redis
sudo apt-get install redis-server
sudo systemctl start redis
```

### 4. Initialize Database

```bash
# Run database migrations
sqlx database create
sqlx migrate run

# Or use our init script
psql $DATABASE_URL -f database/init.sql
```

### 5. Build and Test

```bash
# Build all components
cargo build

# Run tests
cargo test

# Start development server
cargo run --bin ghostllm-proxy -- serve --dev
```

## 📁 Project Structure

```
ghostllm/
├── .github/                    # GitHub workflows and templates
│   ├── workflows/
│   │   ├── ci.yml             # Continuous integration
│   │   ├── release.yml        # Release automation
│   │   └── security.yml       # Security scanning
│   └── ISSUE_TEMPLATE/        # Issue templates
├── apps/                      # Application binaries
│   ├── proxy-server/          # Main HTTP server
│   │   ├── src/
│   │   │   └── main.rs        # Entry point with CLI
│   │   └── Cargo.toml
│   └── tauri-app/            # Desktop application
│       ├── src/              # Rust backend
│       ├── src-tauri/        # Tauri configuration
│       └── Cargo.toml
├── crates/                   # Library crates
│   ├── ghostllm-core/        # Core business logic
│   │   ├── src/
│   │   │   ├── lib.rs        # Public API
│   │   │   ├── types.rs      # Core types
│   │   │   ├── error.rs      # Error handling
│   │   │   ├── config.rs     # Configuration
│   │   │   ├── auth.rs       # Authentication
│   │   │   ├── metrics.rs    # Metrics collection
│   │   │   └── providers/    # Provider implementations
│   │   │       ├── mod.rs    # Provider registry
│   │   │       ├── openai.rs
│   │   │       ├── anthropic.rs
│   │   │       ├── google.rs
│   │   │       └── ollama.rs
│   │   ├── tests/            # Integration tests
│   │   └── Cargo.toml
│   ├── ghostllm-proxy/       # HTTP server implementation
│   │   ├── src/
│   │   │   ├── lib.rs        # Public API
│   │   │   ├── server.rs     # Server implementation
│   │   │   ├── middleware/   # HTTP middleware
│   │   │   │   ├── auth.rs   # Authentication
│   │   │   │   ├── cors.rs   # CORS handling
│   │   │   │   └── rate_limit.rs # Rate limiting
│   │   │   ├── handlers/     # Request handlers
│   │   │   │   ├── health.rs # Health endpoints
│   │   │   │   ├── models.rs # Model management
│   │   │   │   ├── chat.rs   # Chat completions
│   │   │   │   └── admin.rs  # Admin API
│   │   │   └── routes.rs     # Route definitions
│   │   └── Cargo.toml
│   ├── ghostllm-web/         # Web frontend (Yew)
│   │   ├── src/
│   │   │   ├── lib.rs        # App entry point
│   │   │   ├── components/   # UI components
│   │   │   ├── pages/        # Page components
│   │   │   └── services/     # API services
│   │   └── Cargo.toml
│   └── ghostllm-cli/         # CLI implementation
│       ├── src/
│       │   ├── main.rs       # CLI entry point
│       │   └── commands/     # Command handlers
│       └── Cargo.toml
├── database/                 # Database files
│   ├── init.sql             # Initial schema
│   └── migrations/          # SQL migrations
├── docs/                    # Documentation
│   ├── API.md              # API documentation
│   ├── DEPLOYMENT.md       # Deployment guide
│   ├── ARCHITECTURE.md     # Architecture overview
│   └── DEVELOPMENT.md      # This file
├── nginx/                   # Nginx configuration
│   ├── nginx.conf          # Main config
│   └── ssl/                # SSL certificates
├── monitoring/              # Monitoring configs
│   ├── prometheus.yml      # Prometheus config
│   └── grafana/            # Grafana dashboards
├── scripts/                # Utility scripts
│   ├── build.sh           # Build script
│   ├── test.sh            # Test script
│   └── deploy.sh          # Deployment script
├── tests/                  # Integration tests
│   ├── api/               # API tests
│   ├── providers/         # Provider tests
│   └── common/            # Test utilities
├── .env.example           # Environment template
├── .gitignore            # Git ignore rules
├── Boltfile              # Bolt deployment config
├── Cargo.toml            # Workspace configuration
├── docker-compose.yml    # Development services
├── Dockerfile            # Container build
├── Makefile              # Build automation
├── README.md             # Project overview
└── setup.sh              # Automated setup
```

## 🔧 Development Workflow

### Daily Development

```bash
# Start development environment
make dev

# Run with hot reload
make watch

# In another terminal, run tests
cargo test --watch

# Format code
cargo fmt

# Lint code
cargo clippy

# Check for security issues
cargo audit
```

### Build Commands

```bash
# Build all components
make build
cargo build

# Build release version
cargo build --release

# Build specific component
cargo build -p ghostllm-core
cargo build --bin ghostllm-proxy

# Clean build artifacts
make clean
cargo clean
```

### Testing

```bash
# Run all tests
make test
cargo test

# Run specific test module
cargo test providers::openai
cargo test --package ghostllm-core

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration

# Run doc tests
cargo test --doc

# Generate test coverage
cargo tarpaulin --out Html
```

### Code Quality

```bash
# Format code
cargo fmt
cargo fmt --check  # Check without modifying

# Lint code
cargo clippy
cargo clippy -- -D warnings  # Treat warnings as errors

# Check for unused dependencies
cargo machete

# Security audit
cargo audit
cargo audit fix

# Generate documentation
cargo doc --open
cargo doc --no-deps --open  # Local docs only
```

## 🏗️ Adding New Features

### Adding a New Provider

1. **Create provider module:**
```bash
touch crates/ghostllm-core/src/providers/newprovider.rs
```

2. **Implement provider trait:**
```rust
// crates/ghostllm-core/src/providers/newprovider.rs
use crate::providers::{Provider, ProviderError};
use crate::types::{ChatRequest, ChatResponse, Model};
use async_trait::async_trait;

pub struct NewProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl NewProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "https://api.newprovider.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl Provider for NewProvider {
    fn id(&self) -> &str {
        "newprovider"
    }

    fn name(&self) -> &str {
        "New Provider"
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        // Implement health check
        Ok(true)
    }

    async fn list_models(&self) -> Result<Vec<Model>, ProviderError> {
        // Implement model listing
        Ok(vec![])
    }

    async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse, ProviderError> {
        // Implement chat completion
        todo!("Implement chat completion")
    }
}
```

3. **Add to provider registry:**
```rust
// crates/ghostllm-core/src/providers/mod.rs
pub mod newprovider;
pub use newprovider::NewProvider;

// In registry initialization
if config.newprovider.enabled {
    registry.register("newprovider",
        Box::new(NewProvider::new(config.newprovider.api_key.clone())));
}
```

4. **Add configuration:**
```rust
// crates/ghostllm-core/src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProviderConfig {
    pub enabled: bool,
    pub api_key: String,
    pub base_url: Option<String>,
}
```

5. **Add tests:**
```rust
// crates/ghostllm-core/tests/providers/newprovider.rs
use ghostllm_core::providers::NewProvider;

#[tokio::test]
async fn test_new_provider_health() {
    let provider = NewProvider::new("test-key".to_string());
    let result = provider.health_check().await;
    assert!(result.is_ok());
}
```

### Adding New API Endpoints

1. **Create handler:**
```rust
// crates/ghostllm-proxy/src/handlers/new_endpoint.rs
use axum::{extract::State, response::Json, http::StatusCode};
use serde_json::Value;

pub async fn new_endpoint_handler(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    // Implementation
    Ok(Json(serde_json::json!({"message": "success"})))
}
```

2. **Add route:**
```rust
// crates/ghostllm-proxy/src/routes.rs
use crate::handlers::new_endpoint::new_endpoint_handler;

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/api/new-endpoint", get(new_endpoint_handler))
        // ... other routes
}
```

3. **Add tests:**
```rust
// crates/ghostllm-proxy/tests/endpoints/new_endpoint.rs
use axum_test::TestServer;

#[tokio::test]
async fn test_new_endpoint() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server.get("/api/new-endpoint").await;
    response.assert_status_ok();
}
```

## 🧪 Testing Strategy

### Test Types

1. **Unit Tests** - Individual function testing
2. **Integration Tests** - Component interaction testing
3. **End-to-End Tests** - Full system testing
4. **Performance Tests** - Load and stress testing
5. **Security Tests** - Vulnerability testing

### Test Organization

```
tests/
├── unit/              # Unit tests (alongside source)
├── integration/       # Integration tests
│   ├── api/          # API integration tests
│   ├── providers/    # Provider integration tests
│   └── database/     # Database tests
├── e2e/              # End-to-end tests
├── performance/      # Performance tests
├── security/         # Security tests
└── fixtures/         # Test data and fixtures
```

### Test Examples

**Unit Test:**
```rust
// crates/ghostllm-core/src/auth.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_validation() {
        let auth = AuthService::new("secret".to_string());
        let key = "valid-key";
        assert!(auth.validate_key_format(key));
    }

    #[tokio::test]
    async fn test_jwt_generation() {
        let auth = AuthService::new("secret".to_string());
        let token = auth.generate_jwt("user123").await.unwrap();
        assert!(!token.is_empty());
    }
}
```

**Integration Test:**
```rust
// tests/integration/api/health.rs
use ghostllm_proxy::create_app;
use axum_test::TestServer;

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_app(test_config()).await;
    let server = TestServer::new(app).unwrap();

    let response = server.get("/health").await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "healthy");
}
```

**Provider Test:**
```rust
// tests/integration/providers/openai.rs
use ghostllm_core::providers::OpenAIProvider;

#[tokio::test]
async fn test_openai_models() {
    let provider = OpenAIProvider::new(test_config()).await.unwrap();
    let models = provider.list_models().await.unwrap();
    assert!(!models.is_empty());
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test unit
cargo test integration
cargo test --test health

# Run tests with environment
TEST_DATABASE_URL=postgresql://... cargo test

# Run tests in parallel
cargo test -- --test-threads=4

# Run tests with coverage
cargo tarpaulin --all-features --workspace --timeout 120
```

## 🔍 Debugging

### Logging Configuration

```rust
// Enable debug logging
RUST_LOG=debug cargo run

// Enable specific module logging
RUST_LOG=ghostllm_core::providers=debug cargo run

// Enable trace logging
RUST_LOG=trace cargo run
```

### Debugging Tools

```bash
# Debug with GDB
rust-gdb target/debug/ghostllm-proxy

# Memory debugging with Valgrind
valgrind --leak-check=full target/debug/ghostllm-proxy

# Profile with perf
perf record target/release/ghostllm-proxy
perf report

# Flame graph generation
cargo install flamegraph
cargo flamegraph --bin ghostllm-proxy
```

### Common Debug Patterns

```rust
// Debug printing
dbg!(&variable);
println!("Debug: {:?}", variable);

// Conditional compilation for debug
#[cfg(debug_assertions)]
println!("Debug mode only");

// Tracing for async code
use tracing::{info, debug, trace};

#[tracing::instrument]
async fn my_function(param: &str) -> Result<String> {
    debug!("Processing parameter: {}", param);
    // function body
}
```

## 📊 Performance Optimization

### Profiling

```bash
# Install profiling tools
cargo install cargo-profiler
cargo install flamegraph

# Profile CPU usage
cargo profiler callgrind --bin ghostllm-proxy
cargo profiler cachegrind --bin ghostllm-proxy

# Generate flame graphs
cargo flamegraph --bin ghostllm-proxy -- serve --dev

# Memory profiling
cargo profiler memcheck --bin ghostllm-proxy
```

### Benchmarking

```rust
// Criterion benchmarks
// benches/chat_completion.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ghostllm_core::providers::OpenAIProvider;

fn benchmark_chat_completion(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let provider = rt.block_on(async {
        OpenAIProvider::new(test_config()).await.unwrap()
    });

    c.bench_function("chat_completion", |b| {
        b.to_async(&rt).iter(|| async {
            let request = create_test_request();
            provider.chat_completion(black_box(&request)).await
        })
    });
}

criterion_group!(benches, benchmark_chat_completion);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench
cargo bench --bench chat_completion
```

### Memory Optimization

```rust
// Use Arc for shared data
use std::sync::Arc;

// Prefer owned data over cloning
fn process_data(data: String) -> String {
    // Process owned data
    data.to_uppercase()
}

// Use iterators instead of collecting
fn process_items(items: &[Item]) -> impl Iterator<Item = ProcessedItem> + '_ {
    items.iter().map(|item| process_item(item))
}

// Pool connections
use deadpool_postgres::{Config, Pool};

lazy_static! {
    static ref DB_POOL: Pool = create_pool();
}
```

## 🚀 Release Process

### Version Management

```bash
# Update version in Cargo.toml files
# Update CHANGELOG.md
# Create git tag
git tag v0.3.0
git push origin v0.3.0

# Automated via GitHub Actions
# .github/workflows/release.yml handles:
# - Building binaries
# - Creating Docker images
# - Publishing to registries
# - Generating release notes
```

### Pre-release Checklist

- [ ] All tests passing
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version numbers bumped
- [ ] Security audit clean
- [ ] Performance benchmarks stable
- [ ] Integration tests with real providers
- [ ] Docker image builds successfully

## 🤝 Contributing Guidelines

### Code Style

```bash
# Format code before committing
cargo fmt

# Ensure clippy passes
cargo clippy -- -D warnings

# Check for common issues
cargo deny check
```

### Git Workflow

```bash
# Create feature branch
git checkout -b feature/new-provider

# Make changes and commit
git add .
git commit -m "feat: add new provider support"

# Push and create PR
git push origin feature/new-provider
# Create PR via GitHub
```

### Commit Message Convention

```
type(scope): description

feat(providers): add support for new provider
fix(auth): resolve JWT validation issue
docs(api): update endpoint documentation
test(integration): add provider health tests
refactor(core): simplify provider registry
```

### Pull Request Process

1. **Create Issue** - Describe the feature or bug
2. **Create Branch** - Use descriptive name
3. **Implement Changes** - Follow code style
4. **Add Tests** - Ensure good coverage
5. **Update Docs** - Keep documentation current
6. **Create PR** - Link to issue, describe changes
7. **Code Review** - Address feedback
8. **Merge** - Squash commits for clean history

### Review Checklist

- [ ] Code follows style guidelines
- [ ] Tests added for new functionality
- [ ] Documentation updated
- [ ] No breaking changes (or documented)
- [ ] Performance impact considered
- [ ] Security implications reviewed
- [ ] Error handling appropriate

## 🔧 IDE Setup

### VS Code Configuration

**Extensions:**
- `rust-analyzer` - Rust language support
- `CodeLLDB` - Debugging support
- `crates` - Dependency management
- `Better TOML` - TOML file support

**Settings (.vscode/settings.json):**
```json
{
    "rust-analyzer.cargo.allFeatures": true,
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.checkOnSave.allTargets": false,
    "editor.formatOnSave": true,
    "editor.rulers": [100]
}
```

**Tasks (.vscode/tasks.json):**
```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "dev",
            "type": "shell",
            "command": "make",
            "args": ["dev"],
            "group": "build",
            "presentation": {
                "echo": true,
                "reveal": "always",
                "focus": false,
                "panel": "shared"
            }
        }
    ]
}
```

### IntelliJ/CLion Configuration

- Install **Rust** plugin
- Configure **Rust toolchain** in settings
- Set up **run configurations** for different targets
- Enable **Clippy** and **rustfmt** integration

## 📚 Additional Resources

### Learning Resources

- [Rust Book](https://doc.rust-lang.org/book/) - Official Rust documentation
- [Axum Documentation](https://docs.rs/axum/) - Web framework docs
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) - Async runtime
- [PostgreSQL Documentation](https://www.postgresql.org/docs/) - Database docs

### Community

- **Discord** - Join our development channel
- **GitHub Discussions** - Ask questions and share ideas
- **Stack Overflow** - Tag questions with `ghostllm`

### Getting Help

1. **Check Documentation** - README, API docs, architecture
2. **Search Issues** - GitHub issues for similar problems
3. **Create Issue** - Detailed bug report or feature request
4. **Join Discussion** - Community channels for questions

---

**Happy coding! 🦀 Welcome to the GhostLLM development community!**