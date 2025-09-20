# GhostFlow Development Guide

Complete guide for developers wanting to contribute to or extend GhostFlow.

## 🛠️ Development Setup

### Prerequisites

- **Rust 1.75+** with `rustup`
- **Docker & Docker Compose**
- **PostgreSQL 16+** (for native development)
- **Git**
- **Node.js 18+** (for UI development)

### Clone and Setup

```bash
# Clone the repository
git clone https://github.com/ghostkellz/ghostflow
cd ghostflow

# Install Rust toolchain (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install required tools
cargo install sqlx-cli --no-default-features --features postgres
cargo install cargo-watch
cargo install wasm-pack
```

### Development Environment

**Option 1: Full Docker (Recommended for beginners)**
```bash
# Start all services
./scripts/start.sh dev

# View logs
docker-compose logs -f ghostflow
```

**Option 2: Hybrid (Database in Docker, App native)**
```bash
# Start only database services
docker-compose up -d postgres minio ollama

# Set environment variables
export DATABASE_URL="postgresql://ghostflow:ghostflow@localhost:5432/ghostflow"
export RUST_LOG=debug

# Run migrations
sqlx migrate run

# Start development server with hot reload
cargo watch -x "run --bin ghostflow-server"
```

**Option 3: Full Native**
```bash
# Install and start PostgreSQL
sudo apt-get install postgresql postgresql-contrib
sudo systemctl start postgresql
createdb ghostflow

# Install and start Ollama (optional)
curl -fsSL https://ollama.ai/install.sh | sh
ollama serve &
ollama pull llama2

# Run the application
export DATABASE_URL="postgresql://localhost/ghostflow"
cargo run --bin ghostflow-server
```

## 🏗️ Architecture Deep Dive

### Project Structure

```
ghostflow/
├── crates/
│   ├── ghostflow-core/           # Core traits and error types
│   │   ├── src/
│   │   │   ├── error.rs         # Error definitions
│   │   │   ├── traits.rs        # Core traits (Node, NodeRegistry, etc.)
│   │   │   └── lib.rs           # Module exports
│   │   └── Cargo.toml
│   ├── ghostflow-schema/         # Data models and schemas
│   │   ├── src/
│   │   │   ├── flow.rs          # Flow definitions
│   │   │   ├── node.rs          # Node definitions
│   │   │   ├── execution.rs     # Execution models
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── ghostflow-engine/         # Execution engine
│   │   ├── src/
│   │   │   ├── executor.rs      # Flow execution logic
│   │   │   ├── scheduler.rs     # Flow scheduling
│   │   │   ├── runtime.rs       # Runtime management
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── ghostflow-nodes/          # Built-in nodes
│   │   ├── src/
│   │   │   ├── http.rs          # HTTP request node
│   │   │   ├── control_flow.rs  # Control flow nodes
│   │   │   ├── template.rs      # Template processing
│   │   │   ├── webhook.rs       # Webhook trigger
│   │   │   ├── ollama.rs        # Ollama AI nodes
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── ghostflow-api/            # REST/WebSocket API
│   │   ├── src/
│   │   │   ├── routes/          # API route handlers
│   │   │   ├── websocket.rs     # WebSocket handlers
│   │   │   ├── auth.rs          # Authentication
│   │   │   ├── state.rs         # Application state
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── ghostflow-ui/             # Leptos web UI
│   │   ├── src/
│   │   │   ├── components/      # UI components
│   │   │   ├── pages/           # Page components
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── ghostflow-jarvis/         # Jarvis integration
│   │   ├── src/
│   │   │   └── lib.rs           # Jarvis bridge
│   │   └── Cargo.toml
│   ├── ghostflow-server/         # Main server binary
│   │   ├── src/
│   │   │   └── main.rs          # Server entry point
│   │   └── Cargo.toml
│   └── ghostflow-cli/            # CLI tool
│       ├── src/
│       │   ├── main.rs          # CLI entry point
│       │   └── examples/
│       └── Cargo.toml
├── migrations/                   # Database migrations
├── docs/                        # Documentation
├── scripts/                     # Helper scripts
├── docker-compose.yml           # Docker orchestration
├── Dockerfile                   # Multi-stage build
├── Cargo.toml                   # Workspace definition
└── README.md
```

### Data Flow

```
User Input (UI/API)
        │
        ▼
   API Handler
        │
        ▼
  Flow Validator
        │
        ▼
   Flow Engine ──► Node Registry ──► Individual Nodes
        │                               │
        ▼                               ▼
   Database ◄──── Execution Results ◄───┘
        │
        ▼
  WebSocket ──► Real-time Updates ──► UI
```

## 🔧 Development Workflows

### Testing

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p ghostflow-engine

# Run with detailed output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration

# Run with coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Check for security vulnerabilities
cargo audit

# Check for unused dependencies
cargo machete

# Generate documentation
cargo doc --open
```

### Database Development

```bash
# Create new migration
sqlx migrate add create_new_table

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Check migration status
sqlx migrate info

# Prepare offline queries (for CI)
cargo sqlx prepare
```

### UI Development (Leptos)

```bash
# Install trunk for serving WASM
cargo install trunk

# Start development server with hot reload
cd crates/ghostflow-ui
trunk serve --open

# Build for production
trunk build --release
```

## 🔌 Creating Custom Nodes

### Step 1: Define Your Node

Create a new file in `crates/ghostflow-nodes/src/`:

```rust
// crates/ghostflow-nodes/src/my_node.rs
use async_trait::async_trait;
use ghostflow_core::{GhostFlowError, Node, Result};
use ghostflow_schema::*;
use serde_json::Value;
use tracing::info;

pub struct MyCustomNode {
    // Add any configuration fields here
    config: MyNodeConfig,
}

#[derive(Debug, Clone)]
pub struct MyNodeConfig {
    pub default_timeout: std::time::Duration,
}

impl MyCustomNode {
    pub fn new() -> Self {
        Self {
            config: MyNodeConfig {
                default_timeout: std::time::Duration::from_secs(30),
            }
        }
    }

    pub fn with_config(config: MyNodeConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Node for MyCustomNode {
    fn definition(&self) -> NodeDefinition {
        NodeDefinition {
            id: "my_custom_node".to_string(),
            name: "My Custom Node".to_string(),
            description: "This node does something amazing".to_string(),
            category: NodeCategory::Action,
            version: "1.0.0".to_string(),
            inputs: vec![
                NodePort {
                    name: "input_data".to_string(),
                    display_name: "Input Data".to_string(),
                    description: Some("Data to process".to_string()),
                    data_type: DataType::Object,
                    required: true,
                }
            ],
            outputs: vec![
                NodePort {
                    name: "processed_data".to_string(),
                    display_name: "Processed Data".to_string(),
                    description: Some("Processed result".to_string()),
                    data_type: DataType::Object,
                    required: true,
                }
            ],
            parameters: vec![
                NodeParameter {
                    name: "operation".to_string(),
                    display_name: "Operation".to_string(),
                    description: Some("Operation to perform".to_string()),
                    param_type: ParameterType::Select,
                    default_value: Some(Value::String("transform".to_string())),
                    required: true,
                    options: Some(vec![
                        serde_json::from_str(r#"{"value": "transform", "label": "Transform"}"#).unwrap(),
                        serde_json::from_str(r#"{"value": "filter", "label": "Filter"}"#).unwrap(),
                        serde_json::from_str(r#"{"value": "aggregate", "label": "Aggregate"}"#).unwrap(),
                    ]),
                    validation: None,
                },
                NodeParameter {
                    name: "timeout_seconds".to_string(),
                    display_name: "Timeout (seconds)".to_string(),
                    description: Some("Operation timeout".to_string()),
                    param_type: ParameterType::Number,
                    default_value: Some(Value::Number(serde_json::Number::from(30))),
                    required: false,
                    options: None,
                    validation: Some(ParameterValidation {
                        min_value: Some(1.0),
                        max_value: Some(300.0),
                        min_length: None,
                        max_length: None,
                        pattern: None,
                    }),
                },
            ],
            icon: Some("gear".to_string()),
            color: Some("#10b981".to_string()),
        }
    }

    async fn validate(&self, context: &ExecutionContext) -> Result<()> {
        let params = &context.input;
        
        // Validate required inputs
        if params.get("input_data").is_none() {
            return Err(GhostFlowError::ValidationError {
                message: "input_data is required".to_string(),
            });
        }

        // Validate operation parameter
        if let Some(operation) = params.get("operation").and_then(|v| v.as_str()) {
            match operation {
                "transform" | "filter" | "aggregate" => {},
                _ => return Err(GhostFlowError::ValidationError {
                    message: format!("Invalid operation: {}", operation),
                }),
            }
        }

        // Validate timeout
        if let Some(timeout) = params.get("timeout_seconds").and_then(|v| v.as_f64()) {
            if timeout <= 0.0 || timeout > 300.0 {
                return Err(GhostFlowError::ValidationError {
                    message: "Timeout must be between 1 and 300 seconds".to_string(),
                });
            }
        }

        Ok(())
    }

    async fn execute(&self, context: ExecutionContext) -> Result<serde_json::Value> {
        let params = &context.input;
        let start_time = std::time::Instant::now();
        
        info!("Executing custom node for execution {}", context.execution_id);

        // Extract parameters
        let input_data = params.get("input_data").unwrap();
        let operation = params.get("operation").and_then(|v| v.as_str()).unwrap_or("transform");
        let timeout_seconds = params.get("timeout_seconds").and_then(|v| v.as_f64()).unwrap_or(30.0);

        // Perform the operation with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_secs_f64(timeout_seconds),
            self.perform_operation(operation, input_data.clone())
        ).await.map_err(|_| GhostFlowError::TimeoutError {
            timeout_ms: (timeout_seconds * 1000.0) as u64,
        })??;

        let execution_time = start_time.elapsed();
        info!("Custom node completed in {}ms", execution_time.as_millis());

        Ok(serde_json::json!({
            "processed_data": result,
            "operation": operation,
            "execution_time_ms": execution_time.as_millis(),
            "node_id": context.node_id
        }))
    }

    fn supports_retry(&self) -> bool {
        true
    }

    fn is_deterministic(&self) -> bool {
        true // Change to false if your node has non-deterministic behavior
    }
}

impl MyCustomNode {
    async fn perform_operation(&self, operation: &str, data: Value) -> Result<Value> {
        match operation {
            "transform" => self.transform_data(data).await,
            "filter" => self.filter_data(data).await,
            "aggregate" => self.aggregate_data(data).await,
            _ => Err(GhostFlowError::NodeExecutionError {
                node_id: "my_custom_node".to_string(),
                message: format!("Unknown operation: {}", operation),
            }),
        }
    }

    async fn transform_data(&self, data: Value) -> Result<Value> {
        // Your transformation logic here
        // This is just an example
        match data {
            Value::Object(mut obj) => {
                obj.insert("transformed".to_string(), Value::Bool(true));
                obj.insert("timestamp".to_string(), Value::String(chrono::Utc::now().to_rfc3339()));
                Ok(Value::Object(obj))
            }
            _ => Ok(serde_json::json!({
                "original": data,
                "transformed": true,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
    }

    async fn filter_data(&self, data: Value) -> Result<Value> {
        // Your filtering logic here
        Ok(data) // Placeholder
    }

    async fn aggregate_data(&self, data: Value) -> Result<Value> {
        // Your aggregation logic here
        Ok(data) // Placeholder
    }
}

impl Default for MyCustomNode {
    fn default() -> Self {
        Self::new()
    }
}
```

### Step 2: Add to Module

Update `crates/ghostflow-nodes/src/lib.rs`:

```rust
pub mod my_node;
pub use my_node::*;
```

### Step 3: Register the Node

In your server initialization code:

```rust
use ghostflow_nodes::MyCustomNode;

// In your server startup
let mut registry = BasicNodeRegistry::new();
registry.register_node("my_custom_node".to_string(), Arc::new(MyCustomNode::new()))?;
```

### Step 4: Write Tests

Create `crates/ghostflow-nodes/src/my_node.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ghostflow_schema::ExecutionContext;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_node_definition() {
        let node = MyCustomNode::new();
        let def = node.definition();
        
        assert_eq!(def.id, "my_custom_node");
        assert_eq!(def.name, "My Custom Node");
        assert!(def.inputs.len() > 0);
        assert!(def.outputs.len() > 0);
    }

    #[tokio::test]
    async fn test_validation_success() {
        let node = MyCustomNode::new();
        let context = create_test_context(serde_json::json!({
            "input_data": {"test": "data"},
            "operation": "transform"
        }));
        
        let result = node.validate(&context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validation_missing_input() {
        let node = MyCustomNode::new();
        let context = create_test_context(serde_json::json!({
            "operation": "transform"
        }));
        
        let result = node.validate(&context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transform_operation() {
        let node = MyCustomNode::new();
        let context = create_test_context(serde_json::json!({
            "input_data": {"name": "test"},
            "operation": "transform"
        }));
        
        let result = node.execute(context).await.unwrap();
        assert!(result["processed_data"]["transformed"].as_bool().unwrap());
    }

    fn create_test_context(input: serde_json::Value) -> ExecutionContext {
        ExecutionContext {
            execution_id: Uuid::new_v4(),
            flow_id: Uuid::new_v4(),
            node_id: "test_node".to_string(),
            input,
            variables: HashMap::new(),
            secrets: HashMap::new(),
            artifacts: HashMap::new(),
        }
    }
}
```

### Step 5: Run and Test

```bash
# Test your node
cargo test -p ghostflow-nodes test_my_custom_node

# Test integration
cargo run --bin simple_flow
```

## 🎨 UI Development

### Component Structure

```rust
// crates/ghostflow-ui/src/components/flow_editor.rs
use leptos::*;
use ghostflow_schema::Flow;

#[component]
pub fn FlowEditor(flow: ReadSignal<Option<Flow>>) -> impl IntoView {
    view! {
        <div class="flow-editor">
            <div class="toolbar">
                <button>"Save"</button>
                <button>"Run"</button>
                <button>"Export"</button>
            </div>
            <div class="canvas">
                <FlowCanvas flow=flow/>
            </div>
            <div class="sidebar">
                <NodePalette/>
                <PropertyPanel/>
            </div>
        </div>
    }
}

#[component]
fn FlowCanvas(flow: ReadSignal<Option<Flow>>) -> impl IntoView {
    view! {
        <canvas id="flow-canvas" width="800" height="600">
            // Canvas-based flow editor
        </canvas>
    }
}

#[component]
fn NodePalette() -> impl IntoView {
    view! {
        <div class="node-palette">
            <h3>"Available Nodes"</h3>
            <div class="node-categories">
                <NodeCategory name="Actions"/>
                <NodeCategory name="Triggers"/>
                <NodeCategory name="AI/ML"/>
            </div>
        </div>
    }
}
```

### API Integration

```rust
// crates/ghostflow-ui/src/api.rs
use serde_json::Value;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

pub async fn create_flow(flow: &Flow) -> Result<Flow, String> {
    let body = serde_json::to_string(flow).map_err(|e| e.to_string())?;
    
    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&body.into()));
    
    let request = Request::new_with_str_and_init("/api/flows", &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;
    
    let window = web_sys::window().unwrap();
    let response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;
    
    let response: Response = response.dyn_into().unwrap();
    let json = JsFuture::from(response.json().unwrap()).await.unwrap();
    
    // Parse response...
    todo!("Parse JSON response to Flow")
}
```

## 🐛 Debugging

### Logging

```rust
use tracing::{debug, info, warn, error, trace};

// Add to your functions
info!("Starting node execution for {}", context.node_id);
debug!("Input data: {:?}", context.input);
warn!("Deprecated parameter used: {}", param_name);
error!("Node execution failed: {}", error);
```

### Environment Variables

```bash
# Enable debug logging
export RUST_LOG=debug

# Enable trace logging for specific modules
export RUST_LOG=ghostflow_engine=trace,ghostflow_nodes=debug

# Enable backtraces
export RUST_BACKTRACE=1

# For full backtraces
export RUST_BACKTRACE=full
```

### Debugging with VSCode

Create `.vscode/launch.json`:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug GhostFlow Server",
            "cargo": {
                "args": ["build", "--bin=ghostflow-server"],
                "filter": {
                    "name": "ghostflow-server",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}",
            "env": {
                "RUST_LOG": "debug",
                "DATABASE_URL": "postgresql://ghostflow:ghostflow@localhost/ghostflow"
            }
        }
    ]
}
```

## 🔄 Contributing

### Workflow

1. **Fork and Clone**
   ```bash
   git clone https://github.com/YOUR_USERNAME/ghostflow
   cd ghostflow
   git remote add upstream https://github.com/ghostkellz/ghostflow
   ```

2. **Create Feature Branch**
   ```bash
   git checkout -b feature/my-awesome-feature
   ```

3. **Make Changes**
   - Write code
   - Add tests
   - Update documentation
   - Run quality checks

4. **Test Everything**
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

5. **Commit and Push**
   ```bash
   git add .
   git commit -m "feat: add my awesome feature"
   git push origin feature/my-awesome-feature
   ```

6. **Create Pull Request**
   - Go to GitHub
   - Create PR from your branch to `main`
   - Fill out the PR template

### Code Style

- Use `rustfmt` for formatting: `cargo fmt`
- Follow Rust naming conventions
- Add documentation for public APIs
- Write comprehensive tests
- Keep functions focused and small
- Use meaningful variable names

### Commit Messages

Follow conventional commits:
- `feat:` new features
- `fix:` bug fixes
- `docs:` documentation changes
- `test:` adding tests
- `refactor:` code refactoring
- `perf:` performance improvements

## 📊 Performance

### Profiling

```bash
# Install profiling tools
cargo install cargo-profiler
cargo install flamegraph

# Profile the server
cargo flamegraph --bin ghostflow-server

# Memory profiling with valgrind
cargo profiler callgrind --bin ghostflow-server
```

### Benchmarking

```bash
# Add to Cargo.toml
[[bench]]
name = "flow_execution"
harness = false

# Create benches/flow_execution.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_flow_execution(c: &mut Criterion) {
    c.bench_function("simple_flow", |b| {
        b.iter(|| {
            // Your benchmark code
            black_box(execute_simple_flow())
        })
    });
}

criterion_group!(benches, benchmark_flow_execution);
criterion_main!(benches);

# Run benchmarks
cargo bench
```

### Optimization Tips

1. **Use `Arc` and `Rc` wisely** - Avoid unnecessary cloning
2. **Prefer `&str` over `String`** when possible
3. **Use `tokio::spawn` for CPU-bound tasks**
4. **Pool database connections**
5. **Cache frequently accessed data**
6. **Use `serde_json::from_slice` for zero-copy parsing**

---

This development guide should get you started with contributing to GhostFlow. For specific questions, check the GitHub issues or discussions!