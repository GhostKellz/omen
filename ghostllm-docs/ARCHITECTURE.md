# 🏗️ GhostLLM Architecture

Comprehensive architecture documentation for the GhostLLM enterprise LLM proxy.

## 📊 System Overview

GhostLLM is built as a modern, cloud-native application using a microservices-oriented architecture with a focus on scalability, reliability, and maintainability.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Internet                                       │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Load Balancer / CDN                                │
│                    (Nginx, Cloudflare, AWS ALB)                            │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         API Gateway Layer                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │   Web Frontend  │  │  Tauri Desktop  │  │     OpenWebUI Client       │ │
│  │   (Port 80)     │  │   (Port 4433)   │  │    (External Apps)          │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘ │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                     GhostLLM Core Application                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    HTTP Server (Axum)                               │   │
│  │                      Port 8080                                      │   │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │   │
│  │  │   Auth      │ │  Rate Limit │ │   CORS      │ │  Compression│   │   │
│  │  │ Middleware  │ │ Middleware  │ │ Middleware  │ │ Middleware  │   │   │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│  ┌─────────────────────────────────┼─────────────────────────────────────┐  │
│  │              Business Logic Layer                                    │  │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │  │
│  │  │                Provider Registry                                │ │  │
│  │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │ │  │
│  │  │  │ OpenAI  │ │Anthropic│ │ Google  │ │ Ollama  │ │   ...   │  │ │  │
│  │  │  │Provider │ │Provider │ │Provider │ │Provider │ │Provider │  │ │  │
│  │  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘  │ │  │
│  │  └─────────────────────────────────────────────────────────────────┘ │  │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │  │
│  │  │    User     │ │   API Key   │ │   Usage     │ │   Config    │   │  │
│  │  │ Management  │ │ Management  │ │  Tracking   │ │ Management  │   │  │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Data Layer                                        │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │   PostgreSQL    │  │      Redis      │  │        File Storage         │ │
│  │   (Database)    │  │     (Cache)     │  │    (Logs, Configs, SSL)     │ │
│  │                 │  │                 │  │                             │ │
│  │ • Users         │  │ • Rate Limits   │  │ • Application Logs          │ │
│  │ • API Keys      │  │ • Cache Data    │  │ • SSL Certificates          │ │
│  │ • Providers     │  │ • Sessions      │  │ • Static Assets             │ │
│  │ • Models        │  │ • Temp Data     │  │ • Backup Files              │ │
│  │ • Usage Logs    │  │                 │  │                             │ │
│  │ • Chat Sessions │  │                 │  │                             │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      External Provider APIs                                │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │   OpenAI    │ │ Anthropic   │ │   Google    │ │     Local Ollama    │   │
│  │     API     │ │     API     │ │     API     │ │        Server       │   │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🔧 Component Architecture

### Core Application Structure

```
ghostllm/
├── apps/
│   ├── proxy-server/          # Main HTTP server application
│   │   ├── src/
│   │   │   ├── main.rs        # Entry point and CLI handling
│   │   │   └── config.rs      # Configuration management
│   │   └── Cargo.toml
│   │
│   └── tauri-app/             # Desktop management application
│       ├── src/
│       │   ├── main.rs        # Tauri app entry point
│       │   └── commands.rs    # Tauri command handlers
│       ├── src-tauri/         # Tauri configuration
│       └── Cargo.toml
│
├── crates/
│   ├── ghostllm-core/         # Core business logic
│   │   ├── src/
│   │   │   ├── lib.rs         # Public API exports
│   │   │   ├── types.rs       # Core type definitions
│   │   │   ├── error.rs       # Error handling
│   │   │   ├── config.rs      # Configuration types
│   │   │   ├── auth.rs        # Authentication logic
│   │   │   ├── metrics.rs     # Metrics collection
│   │   │   └── providers/     # Provider implementations
│   │   │       ├── mod.rs     # Provider traits and registry
│   │   │       ├── openai.rs  # OpenAI provider
│   │   │       ├── anthropic.rs # Anthropic provider
│   │   │       ├── google.rs  # Google provider
│   │   │       └── ollama.rs  # Ollama provider
│   │   └── Cargo.toml
│   │
│   ├── ghostllm-proxy/        # HTTP server implementation
│   │   ├── src/
│   │   │   ├── lib.rs         # Server public API
│   │   │   ├── server.rs      # Main server implementation
│   │   │   ├── middleware/    # HTTP middleware
│   │   │   │   ├── mod.rs
│   │   │   │   ├── auth.rs    # Authentication middleware
│   │   │   │   ├── cors.rs    # CORS handling
│   │   │   │   ├── rate_limit.rs # Rate limiting
│   │   │   │   └── metrics.rs # Metrics middleware
│   │   │   ├── handlers/      # HTTP request handlers
│   │   │   │   ├── mod.rs
│   │   │   │   ├── health.rs  # Health check endpoints
│   │   │   │   ├── models.rs  # Model listing
│   │   │   │   ├── chat.rs    # Chat completions
│   │   │   │   └── admin.rs   # Admin endpoints
│   │   │   └── routes.rs      # Route definitions
│   │   └── Cargo.toml
│   │
│   ├── ghostllm-web/          # Web frontend (Yew)
│   │   ├── src/
│   │   │   ├── lib.rs         # App entry point
│   │   │   ├── components/    # Reusable UI components
│   │   │   ├── pages/         # Page components
│   │   │   └── services/      # API clients
│   │   └── Cargo.toml
│   │
│   └── ghostllm-cli/          # Command-line interface
│       ├── src/
│       │   ├── main.rs        # CLI entry point
│       │   ├── commands/      # CLI command implementations
│       │   └── utils.rs       # CLI utilities
│       └── Cargo.toml
│
└── database/
    ├── init.sql               # Database schema
    └── migrations/            # Database migrations
```

## 🔄 Data Flow

### Request Processing Flow

```
1. HTTP Request
   ↓
2. Load Balancer (Nginx)
   ↓
3. GhostLLM Server (Axum)
   ↓
4. Middleware Stack:
   ├── Compression
   ├── CORS
   ├── Rate Limiting (Redis Check)
   ├── Authentication (JWT/API Key)
   └── Metrics Collection
   ↓
5. Route Handler
   ↓
6. Business Logic:
   ├── Request Validation
   ├── Provider Selection
   ├── Model Availability Check
   └── Usage Tracking
   ↓
7. Provider Communication:
   ├── Request Translation
   ├── HTTP Client (Reqwest)
   ├── External API Call
   └── Response Translation
   ↓
8. Response Processing:
   ├── Usage Logging (Database)
   ├── Caching (Redis)
   ├── Metrics Update
   └── Response Formatting
   ↓
9. HTTP Response
```

### Streaming Data Flow

```
WebSocket Connection:
1. Client Connection
   ↓
2. WebSocket Upgrade (Axum)
   ↓
3. Authentication Check
   ↓
4. Stream Handler:
   ├── Parse Request
   ├── Select Provider
   └── Create Stream
   ↓
5. Provider Streaming:
   ├── HTTP/2 Stream (or SSE)
   ├── Chunk Processing
   └── Real-time Forward
   ↓
6. Client Receives:
   ├── Incremental Tokens
   ├── Usage Updates
   └── Completion Signal
```

## 🧱 Component Details

### Core Library (`ghostllm-core`)

**Responsibilities:**
- Type definitions for all domain objects
- Provider trait definition and registry
- Authentication and authorization logic
- Configuration management
- Error handling and result types
- Metrics collection interfaces

**Key Traits:**
```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn health_check(&self) -> Result<bool>;
    async fn list_models(&self) -> Result<Vec<Model>>;
    async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse>;
    async fn stream_completion(&self, request: &ChatRequest) -> Result<impl Stream<Item = ChatChunk>>;
}

pub trait AuthService: Send + Sync {
    async fn validate_api_key(&self, key: &str) -> Result<User>;
    async fn validate_jwt(&self, token: &str) -> Result<Claims>;
    async fn create_api_key(&self, user_id: &str, permissions: &Permissions) -> Result<ApiKey>;
}
```

### Proxy Server (`ghostllm-proxy`)

**Responsibilities:**
- HTTP server implementation using Axum
- Request routing and middleware
- Rate limiting and caching
- WebSocket handling for streaming
- Health monitoring and metrics exposure

**Key Components:**
```rust
pub struct ProxyServer {
    config: Arc<AppConfig>,
    provider_registry: Arc<ProviderRegistry>,
    auth_service: Arc<dyn AuthService>,
    rate_limiter: Arc<RateLimiter>,
    cache: Arc<Cache>,
}

impl ProxyServer {
    pub async fn new(config: AppConfig) -> Result<Self>;
    pub async fn start(&self) -> Result<()>;
    pub fn create_app(&self) -> Router;
}
```

### Web Frontend (`ghostllm-web`)

**Responsibilities:**
- Modern web UI built with Yew (Rust WebAssembly)
- Provider management interface
- User and API key management
- Usage analytics and dashboards
- Real-time monitoring displays

**Architecture:**
```rust
// Component hierarchy
App
├── Layout
│   ├── Header
│   ├── Navigation
│   └── Footer
├── Pages
│   ├── Dashboard
│   │   ├── MetricsCard
│   │   ├── UsageChart
│   │   └── RecentActivity
│   ├── Providers
│   │   ├── ProviderList
│   │   ├── ProviderForm
│   │   └── ModelList
│   ├── Users
│   │   ├── UserList
│   │   ├── UserForm
│   │   └── ApiKeyManager
│   └── Settings
│       ├── ConfigEditor
│       └── SystemInfo
└── Services
    ├── ApiClient
    ├── WebSocketService
    └── NotificationService
```

### CLI Tool (`ghostllm-cli`)

**Responsibilities:**
- Administrative command-line interface
- Configuration management
- Provider testing and diagnostics
- User and API key management
- Database operations and migrations

**Commands:**
```rust
pub enum Commands {
    Serve { /* server options */ },
    Config { /* config operations */ },
    User { /* user management */ },
    Provider { /* provider management */ },
    Test { /* testing and diagnostics */ },
    Migrate { /* database operations */ },
}
```

## 🔐 Security Architecture

### Authentication & Authorization

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Authentication Flow                                 │
│                                                                             │
│  Client Request                                                             │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────┐                                                       │
│  │  API Key Check  │ ─── No Key ───► Reject (401)                          │
│  └─────────────────┘                                                       │
│       │ Has Key                                                             │
│       ▼                                                                     │
│  ┌─────────────────┐                                                       │
│  │ Key Validation  │ ─── Invalid ──► Reject (401)                          │
│  │   (Database)    │                                                       │
│  └─────────────────┘                                                       │
│       │ Valid                                                               │
│       ▼                                                                     │
│  ┌─────────────────┐                                                       │
│  │ Permission      │ ─── Denied ───► Reject (403)                          │
│  │ Check           │                                                       │
│  └─────────────────┘                                                       │
│       │ Allowed                                                             │
│       ▼                                                                     │
│  ┌─────────────────┐                                                       │
│  │ Rate Limit      │ ─── Exceeded ─► Reject (429)                          │
│  │ Check (Redis)   │                                                       │
│  └─────────────────┘                                                       │
│       │ OK                                                                  │
│       ▼                                                                     │
│    Process Request                                                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Security Layers

1. **Transport Security**
   - TLS 1.2+ for all external communication
   - Certificate management with automatic renewal
   - HSTS headers for browser security

2. **Authentication**
   - JWT tokens for user sessions
   - API keys for service-to-service communication
   - bcrypt for password hashing
   - Rate limiting per key/user

3. **Authorization**
   - Role-based access control (RBAC)
   - Permission-based API access
   - Resource-level permissions
   - Budget and usage limits

4. **Data Protection**
   - Encrypted sensitive data at rest
   - API key encryption in database
   - Audit logging for compliance
   - PII data handling compliance

## 📊 Scalability Architecture

### Horizontal Scaling

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Load Balancer                                     │
│                         (Nginx / ALB)                                      │
└─────────────────┬───────────────┬───────────────┬───────────────────────────┘
                  │               │               │
                  ▼               ▼               ▼
┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐
│   GhostLLM Pod 1    │ │   GhostLLM Pod 2    │ │   GhostLLM Pod N    │
│                     │ │                     │ │                     │
│ ┌─────────────────┐ │ │ ┌─────────────────┐ │ │ ┌─────────────────┐ │
│ │ Proxy Server    │ │ │ │ Proxy Server    │ │ │ │ Proxy Server    │ │
│ │ (Stateless)     │ │ │ │ (Stateless)     │ │ │ │ (Stateless)     │ │
│ └─────────────────┘ │ │ └─────────────────┘ │ │ └─────────────────┘ │
└─────────────────────┘ └─────────────────────┘ └─────────────────────┘
          │                       │                       │
          └───────────────────────┼───────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Shared Data Layer                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │   PostgreSQL    │  │   Redis Cluster │  │     Shared Storage          │ │
│  │   (Master/Slave)│  │   (HA Setup)    │  │   (NFS/S3/GCS)              │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Performance Characteristics

| Component | Bottleneck | Scaling Strategy |
|-----------|------------|------------------|
| **HTTP Server** | CPU, Network | Horizontal pods, load balancing |
| **Database** | I/O, Connections | Read replicas, connection pooling |
| **Redis** | Memory, Network | Clustering, sharding |
| **Provider APIs** | Rate limits | Provider selection, caching |

### Caching Strategy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Caching Layers                                  │
│                                                                             │
│  ┌─────────────────┐                                                       │
│  │   CDN Cache     │ ───► Static assets, images                            │
│  │   (Cloudflare)  │      TTL: 24h                                         │
│  └─────────────────┘                                                       │
│          │                                                                 │
│          ▼                                                                 │
│  ┌─────────────────┐                                                       │
│  │  Application    │ ───► Model lists, provider status                     │
│  │     Cache       │      TTL: 5m                                          │
│  │   (In-Memory)   │                                                       │
│  └─────────────────┘                                                       │
│          │                                                                 │
│          ▼                                                                 │
│  ┌─────────────────┐                                                       │
│  │   Redis Cache   │ ───► Chat responses, rate limit counters              │
│  │  (Distributed)  │      TTL: 1h (responses), 1m (counters)              │
│  └─────────────────┘                                                       │
│          │                                                                 │
│          ▼                                                                 │
│  ┌─────────────────┐                                                       │
│  │   Database      │ ───► Persistent data                                  │
│  │   (PostgreSQL)  │      No TTL                                           │
│  └─────────────────┘                                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🔄 Provider Integration Architecture

### Provider Abstraction

```rust
// Trait definition for all providers
#[async_trait]
pub trait Provider: Send + Sync + Debug {
    // Identification
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn provider_type(&self) -> ProviderType;

    // Health and capabilities
    async fn health_check(&self) -> Result<bool>;
    async fn list_models(&self) -> Result<Vec<Model>>;

    // Core functionality
    async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse>;
    async fn stream_completion(&self, request: &ChatRequest) -> Result<impl Stream<Item = ChatChunk>>;

    // Optional capabilities
    async fn text_completion(&self, request: &TextRequest) -> Result<TextResponse> {
        Err(Error::NotSupported("text_completion".to_string()))
    }

    async fn embeddings(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(Error::NotSupported("embeddings".to_string()))
    }
}
```

### Provider Registry

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
    config: ProviderConfig,
}

impl ProviderRegistry {
    pub async fn new(config: ProviderConfig) -> Result<Self> {
        let mut providers = HashMap::new();

        // Initialize providers based on configuration
        if config.openai.enabled {
            providers.insert("openai".to_string(),
                Arc::new(OpenAIProvider::new(config.openai.clone()).await?));
        }

        if config.anthropic.enabled {
            providers.insert("anthropic".to_string(),
                Arc::new(AnthropicProvider::new(config.anthropic.clone()).await?));
        }

        // ... other providers

        Ok(Self { providers, config })
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(id).cloned()
    }

    pub fn all(&self) -> Vec<Arc<dyn Provider>> {
        self.providers.values().cloned().collect()
    }

    pub async fn select_provider(&self, model: &str) -> Result<Arc<dyn Provider>> {
        // Provider selection logic based on model availability,
        // health status, cost, and routing preferences
        todo!()
    }
}
```

## 📈 Monitoring & Observability

### Metrics Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Metrics Collection                                  │
│                                                                             │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────────────┐ │
│  │  Application    │    │   System        │    │    Business             │ │
│  │   Metrics       │    │   Metrics       │    │    Metrics              │ │
│  │                 │    │                 │    │                         │ │
│  │ • Request rate  │    │ • CPU usage     │    │ • Token usage           │ │
│  │ • Latency       │    │ • Memory usage  │    │ • Cost tracking         │ │
│  │ • Error rate    │    │ • Disk I/O      │    │ • User activity         │ │
│  │ • Provider      │    │ • Network I/O   │    │ • Provider performance  │ │
│  │   health        │    │ • Process count │    │ • Model popularity      │ │
│  └─────────────────┘    └─────────────────┘    └─────────────────────────┘ │
│          │                       │                       │                 │
│          └───────────────────────┼───────────────────────┘                 │
│                                  │                                         │
│                                  ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      Prometheus                                     │   │
│  │               (Metrics Storage & Alerting)                          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                  │                                         │
│                                  ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        Grafana                                      │   │
│  │                  (Visualization & Dashboards)                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Logging Architecture

```
Application Logs ──► File System ──► Fluentd/Vector ──► Elasticsearch ──► Kibana
                 │                                  │
                 ├─► Stdout ──────────────────────┐ │
                 │                                │ │
                 └─► Syslog ──────────────────────┘ │
                                                    │
                                                    ▼
                                            Log Aggregation
                                            • Structured JSON
                                            • Request tracing
                                            • Error tracking
                                            • Audit trails
```

## 🔄 Database Schema

### Entity Relationship Diagram

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│      Users      │     │    API Keys     │     │   Providers     │
├─────────────────┤     ├─────────────────┤     ├─────────────────┤
│ id (UUID) PK    │────┐│ id (UUID) PK    │     │ id (STR) PK     │
│ email (UNIQUE)  │    ││ user_id FK      │     │ name            │
│ username        │    ││ key_hash        │     │ provider_type   │
│ password_hash   │    ││ permissions     │     │ base_url        │
│ role            │    ││ rate_limit      │     │ api_key_enc     │
│ budget_limit    │    ││ budget_limit    │     │ enabled         │
│ current_spend   │    ││ current_spend   │     │ config          │
│ created_at      │    ││ expires_at      │     │ created_at      │
│ updated_at      │    ││ last_used_at    │     │ updated_at      │
└─────────────────┘    │└─────────────────┘     └─────────────────┘
         │              │                                 │
         │              └─────────────────────────────────┼─────────┐
         │                                                │         │
         ▼                                                ▼         │
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐ │
│ Chat Sessions   │     │    Messages     │     │     Models      │ │
├─────────────────┤     ├─────────────────┤     ├─────────────────┤ │
│ id (UUID) PK    │────┐│ id (UUID) PK    │     │ id (STR) PK     │ │
│ user_id FK      │    ││ session_id FK   │     │ provider_id FK  │─┘
│ title           │    ││ role            │     │ name            │
│ model_id        │    ││ content         │     │ context_length  │
│ created_at      │    ││ attachments     │     │ input_price     │
│ updated_at      │    ││ created_at      │     │ output_price    │
│ archived        │    │└─────────────────┘     │ supports_vision │
└─────────────────┘    │                       │ supports_funcs  │
         │              │                       │ supports_stream │
         │              │                       │ created_at      │
         │              │                       │ updated_at      │
         ▼              │                       └─────────────────┘
┌─────────────────┐     │
│   Usage Logs    │     │
├─────────────────┤     │
│ id (UUID) PK    │     │
│ user_id FK      │─────┘
│ api_key_id FK   │
│ session_id FK   │
│ model_id        │
│ provider_id     │
│ prompt_tokens   │
│ completion_tkns │
│ total_tokens    │
│ cost            │
│ request_id      │
│ ip_address      │
│ user_agent      │
│ created_at      │
└─────────────────┘
```

### Key Indexes

```sql
-- Performance indexes
CREATE INDEX idx_usage_logs_user_created ON usage_logs(user_id, created_at);
CREATE INDEX idx_usage_logs_model_created ON usage_logs(model_id, created_at);
CREATE INDEX idx_api_keys_user_active ON api_keys(user_id, is_active);
CREATE INDEX idx_chat_sessions_user_updated ON chat_sessions(user_id, updated_at);
CREATE INDEX idx_messages_session_created ON messages(session_id, created_at);

-- Unique constraints
CREATE UNIQUE INDEX idx_users_email ON users(email);
CREATE UNIQUE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE UNIQUE INDEX idx_providers_id ON providers(id);
```

## 🎯 Future Architecture Considerations

### Microservices Evolution

```
Current Monolith:
┌─────────────────────────────────────────────────────────────────────────────┐
│                          GhostLLM Monolith                                 │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │    Auth     │ │   Proxy     │ │  Analytics  │ │      Providers      │   │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘

Future Microservices:
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  Auth Service   │ │  Proxy Service  │ │Analytics Service│ │Provider Service │
│                 │ │                 │ │                 │ │                 │
│ • JWT tokens    │ │ • Request       │ │ • Usage         │ │ • Provider      │
│ • API keys      │ │   routing       │ │   tracking      │ │   health        │
│ • Permissions   │ │ • Rate limiting │ │ • Billing       │ │ • Model lists   │
│ • User mgmt     │ │ • Caching       │ │ • Metrics       │ │ • Load balance  │
└─────────────────┘ └─────────────────┘ └─────────────────┘ └─────────────────┘
```

### Event-Driven Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Event Streaming (Kafka/NATS)                        │
└─────────────────────────────────────────────────────────────────────────────┘
    │                    │                    │                    │
    ▼                    ▼                    ▼                    ▼
┌─────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   Events    │ │     Events      │ │     Events      │ │     Events      │
│             │ │                 │ │                 │ │                 │
│• user.login │ │• request.start  │ │• usage.tracked  │ │• provider.down  │
│• key.created│ │• request.end    │ │• billing.update │ │• model.added    │
│• user.limit │ │• rate.exceeded  │ │• quota.warning  │ │• health.check   │
└─────────────┘ └─────────────────┘ └─────────────────┘ └─────────────────┘
```

---

**This architecture enables enterprise-scale deployment with high availability, horizontal scalability, and comprehensive observability while maintaining simplicity for development and deployment.**