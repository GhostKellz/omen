# 🤖 GhostFlow ↔ Jarvis AI Integration Guide

**Connecting GhostFlow workflow engine with Jarvis AI agent system**

---

## 🌟 Overview

This guide demonstrates how to integrate **Jarvis AI agents** (`github.com/ghostkellz/jarvis`) with **GhostFlow** (`github.com/ghostkellz/ghostflow`) to create powerful automated workflows powered by intelligent AI agents.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    GhostFlow                            │
│  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │  Workflow       │  │       Node Engine           │  │
│  │  Designer       │  │  ┌─────────┐ ┌─────────────┐ │  │
│  │  (Leptos UI)    │  │  │ Jarvis  │ │  GhostLLM   │ │  │
│  │                 │  │  │ Agent   │ │    Node     │ │  │
│  │                 │  │  │ Nodes   │ │             │ │  │
│  └─────────────────┘  │  └─────────┘ └─────────────┘ │  │
└─────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────┐
│                  Jarvis AI System                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐   │
│  │   Agent     │ │  Task       │ │   Knowledge     │   │
│  │  Registry   │ │ Scheduler   │ │     Base        │   │
│  └─────────────┘ └─────────────┘ └─────────────────┘   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐   │
│  │ Reasoning   │ │   Memory    │ │   External      │   │
│  │  Engine     │ │  Manager    │ │   Connectors    │   │
│  └─────────────┘ └─────────────┘ └─────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

---

## 🚀 Quick Start Integration

### 1. Add Jarvis Dependency

Add Jarvis to your GhostFlow project:

```toml
# crates/ghostflow-nodes/Cargo.toml
[dependencies]
jarvis-core = { git = "https://github.com/ghostkellz/jarvis", branch = "main" }
jarvis-agents = { git = "https://github.com/ghostkellz/jarvis", branch = "main" }
jarvis-scheduler = { git = "https://github.com/ghostkellz/jarvis", branch = "main" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
```

### 2. Create Jarvis Agent Node

```rust
// crates/ghostflow-nodes/src/jarvis.rs
use async_trait::async_trait;
use ghostflow_core::{GhostFlowError, Node, Result};
use ghostflow_schema::{
    DataType, ExecutionContext, NodeCategory, NodeDefinition, NodeParameter, NodePort,
};
use ghostflow_schema::node::ParameterType;
use jarvis_core::{Agent, AgentConfig, Task, TaskResult};
use jarvis_agents::{AgentRegistry, AgentType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JarvisNodeConfig {
    pub default_agent_type: AgentType,
    pub max_execution_time: u64, // seconds
    pub enable_memory: bool,
    pub knowledge_base_path: Option<String>,
}

impl Default for JarvisNodeConfig {
    fn default() -> Self {
        Self {
            default_agent_type: AgentType::General,
            max_execution_time: 300, // 5 minutes
            enable_memory: true,
            knowledge_base_path: Some("./knowledge".to_string()),
        }
    }
}

pub struct JarvisAgentNode {
    agent_registry: Arc<RwLock<AgentRegistry>>,
    config: JarvisNodeConfig,
}

impl JarvisAgentNode {
    pub async fn new() -> Self {
        let agent_registry = Arc::new(RwLock::new(AgentRegistry::new().await));
        Self {
            agent_registry,
            config: JarvisNodeConfig::default(),
        }
    }

    pub async fn with_config(config: JarvisNodeConfig) -> Self {
        let agent_registry = Arc::new(RwLock::new(AgentRegistry::new().await));
        Self { agent_registry, config }
    }
}

#[async_trait]
impl Node for JarvisAgentNode {
    fn definition(&self) -> NodeDefinition {
        NodeDefinition {
            id: "jarvis_agent".to_string(),
            name: "Jarvis AI Agent".to_string(),
            description: "Execute tasks using intelligent Jarvis AI agents with reasoning capabilities".to_string(),
            category: NodeCategory::Ai,
            version: "1.0.0".to_string(),
            inputs: vec![
                NodePort {
                    name: "task".to_string(),
                    display_name: "Task".to_string(),
                    description: Some("Task description or instruction for the agent".to_string()),
                    data_type: DataType::String,
                    required: true,
                },
                NodePort {
                    name: "context".to_string(),
                    display_name: "Context".to_string(),
                    description: Some("Additional context or data for the task".to_string()),
                    data_type: DataType::Object,
                    required: false,
                },
            ],
            outputs: vec![
                NodePort {
                    name: "result".to_string(),
                    display_name: "Agent Result".to_string(),
                    description: Some("Result from the AI agent execution".to_string()),
                    data_type: DataType::Object,
                    required: true,
                },
            ],
            parameters: vec![
                NodeParameter {
                    name: "agent_type".to_string(),
                    display_name: "Agent Type".to_string(),
                    description: Some("Type of Jarvis agent to use".to_string()),
                    param_type: ParameterType::String,
                    default_value: Some(Value::String("general".to_string())),
                    required: true,
                    options: Some(vec![
                        Value::String("general".to_string()),
                        Value::String("researcher".to_string()),
                        Value::String("coder".to_string()),
                        Value::String("analyst".to_string()),
                        Value::String("writer".to_string()),
                    ]),
                    validation: None,
                },
                NodeParameter {
                    name: "max_iterations".to_string(),
                    display_name: "Max Iterations".to_string(),
                    description: Some("Maximum reasoning iterations for complex tasks".to_string()),
                    param_type: ParameterType::Number,
                    default_value: Some(Value::Number(serde_json::Number::from(5))),
                    required: false,
                    options: None,
                    validation: None,
                },
                NodeParameter {
                    name: "temperature".to_string(),
                    display_name: "Reasoning Temperature".to_string(),
                    description: Some("Creativity level for agent reasoning (0.0-1.0)".to_string()),
                    param_type: ParameterType::Number,
                    default_value: Some(Value::Number(serde_json::Number::from_f64(0.7).unwrap())),
                    required: false,
                    options: None,
                    validation: None,
                },
                NodeParameter {
                    name: "enable_tools".to_string(),
                    display_name: "Enable External Tools".to_string(),
                    description: Some("Allow agent to use external tools and APIs".to_string()),
                    param_type: ParameterType::Boolean,
                    default_value: Some(Value::Bool(true)),
                    required: false,
                    options: None,
                    validation: None,
                },
            ],
            icon: Some("brain".to_string()),
            color: Some("#f59e0b".to_string()), // Amber for AI agents
        }
    }

    async fn validate(&self, context: &ExecutionContext) -> Result<()> {
        let params = &context.input;
        
        // Validate task input
        if params.get("task").and_then(|v| v.as_str()).map(|s| s.is_empty()).unwrap_or(true) {
            return Err(GhostFlowError::ValidationError {
                message: "Task parameter is required and cannot be empty".to_string(),
            });
        }

        // Validate agent type
        if let Some(agent_type) = params.get("agent_type").and_then(|v| v.as_str()) {
            let valid_types = ["general", "researcher", "coder", "analyst", "writer"];
            if !valid_types.contains(&agent_type) {
                return Err(GhostFlowError::ValidationError {
                    message: format!("Invalid agent type: {}. Must be one of: {:?}", agent_type, valid_types),
                });
            }
        }

        // Validate max_iterations
        if let Some(max_iter) = params.get("max_iterations").and_then(|v| v.as_u64()) {
            if max_iter == 0 || max_iter > 20 {
                return Err(GhostFlowError::ValidationError {
                    message: "Max iterations must be between 1 and 20".to_string(),
                });
            }
        }

        // Validate temperature
        if let Some(temp) = params.get("temperature").and_then(|v| v.as_f64()) {
            if temp < 0.0 || temp > 1.0 {
                return Err(GhostFlowError::ValidationError {
                    message: "Temperature must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        Ok(())
    }

    async fn execute(&self, context: ExecutionContext) -> Result<serde_json::Value> {
        let params = &context.input;
        
        let task_description = params
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GhostFlowError::NodeExecutionError {
                node_id: context.node_id.clone(),
                message: "Missing task parameter".to_string(),
            })?;

        let agent_type_str = params
            .get("agent_type")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        let agent_type = match agent_type_str {
            "general" => AgentType::General,
            "researcher" => AgentType::Researcher,
            "coder" => AgentType::Coder,
            "analyst" => AgentType::Analyst,
            "writer" => AgentType::Writer,
            _ => AgentType::General,
        };

        let max_iterations = params
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        let temperature = params
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7) as f32;

        let enable_tools = params
            .get("enable_tools")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let task_context = params
            .get("context")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        info!(
            "Executing Jarvis agent - type: {}, task: '{}', iterations: {}", 
            agent_type_str, task_description, max_iterations
        );

        let start_time = std::time::Instant::now();
        
        // Get agent from registry
        let agent_registry = self.agent_registry.read().await;
        let agent = agent_registry.get_agent(agent_type).await
            .map_err(|e| GhostFlowError::NodeExecutionError {
                node_id: context.node_id.clone(),
                message: format!("Failed to get agent: {}", e),
            })?;

        // Configure agent for this task
        let agent_config = AgentConfig {
            max_iterations,
            temperature,
            enable_tools,
            enable_memory: self.config.enable_memory,
            timeout: std::time::Duration::from_secs(self.config.max_execution_time),
        };

        agent.configure(agent_config).await;

        // Create and execute task
        let task = Task {
            id: Uuid::new_v4(),
            description: task_description.to_string(),
            context: task_context,
            priority: 1,
            created_at: chrono::Utc::now(),
        };

        let task_result = agent.execute_task(task).await
            .map_err(|e| {
                error!("Jarvis agent execution failed: {}", e);
                GhostFlowError::NodeExecutionError {
                    node_id: context.node_id.clone(),
                    message: format!("Agent task execution failed: {}", e),
                }
            })?;

        let execution_time = start_time.elapsed();

        info!(
            "Jarvis agent completed - duration: {:.2}s, iterations: {}", 
            execution_time.as_secs_f64(),
            task_result.iterations_used
        );

        Ok(serde_json::json!({
            "success": task_result.success,
            "result": task_result.result,
            "reasoning": task_result.reasoning_steps,
            "tools_used": task_result.tools_used,
            "metadata": {
                "agent_type": agent_type_str,
                "task_id": task_result.task_id,
                "iterations_used": task_result.iterations_used,
                "max_iterations": max_iterations,
                "execution_time_ms": execution_time.as_millis(),
                "temperature": temperature,
                "tools_enabled": enable_tools,
                "memory_enabled": self.config.enable_memory,
                "confidence_score": task_result.confidence,
            }
        }))
    }

    fn supports_retry(&self) -> bool {
        true
    }

    fn is_deterministic(&self) -> bool {
        false // AI agents are non-deterministic
    }
}
```

### 3. Add to Node Registry

```rust
// crates/ghostflow-nodes/src/lib.rs
pub mod jarvis;
pub use jarvis::*;
```

---

## 🔧 Advanced Integration Patterns

### Multi-Agent Workflows

Create complex workflows with multiple specialized agents:

```json
{
  "workflow": "research_and_write",
  "nodes": [
    {
      "id": "research_agent",
      "type": "jarvis_agent",
      "config": {
        "agent_type": "researcher",
        "task": "Research the latest trends in AI automation",
        "enable_tools": true
      }
    },
    {
      "id": "analyst_agent", 
      "type": "jarvis_agent",
      "config": {
        "agent_type": "analyst",
        "task": "Analyze research findings and extract key insights",
        "context": "{{research_agent.result}}"
      }
    },
    {
      "id": "writer_agent",
      "type": "jarvis_agent", 
      "config": {
        "agent_type": "writer",
        "task": "Write comprehensive report based on analysis",
        "context": "{{analyst_agent.result}}"
      }
    }
  ]
}
```

### Agent + GhostLLM Pipeline

Combine Jarvis reasoning with GhostLLM inference:

```json
{
  "workflow": "intelligent_content_generation",
  "nodes": [
    {
      "id": "planner_agent",
      "type": "jarvis_agent",
      "config": {
        "agent_type": "general",
        "task": "Plan content structure for: {{user_input}}",
        "max_iterations": 3
      }
    },
    {
      "id": "content_generator",
      "type": "ghostllm_generate",
      "config": {
        "prompt": "{{planner_agent.result.plan}}",
        "temperature": 0.8,
        "max_tokens": 1000
      }
    },
    {
      "id": "reviewer_agent",
      "type": "jarvis_agent",
      "config": {
        "agent_type": "analyst", 
        "task": "Review and improve generated content",
        "context": "{{content_generator.text}}"
      }
    }
  ]
}
```

### Code Generation Workflow

```json
{
  "workflow": "ai_code_generation",
  "nodes": [
    {
      "id": "requirements_agent",
      "type": "jarvis_agent",
      "config": {
        "agent_type": "analyst",
        "task": "Analyze requirements: {{user_requirements}}",
        "enable_tools": true
      }
    },
    {
      "id": "architect_agent", 
      "type": "jarvis_agent",
      "config": {
        "agent_type": "coder",
        "task": "Design system architecture",
        "context": "{{requirements_agent.result}}"
      }
    },
    {
      "id": "implementation_agent",
      "type": "jarvis_agent",
      "config": {
        "agent_type": "coder", 
        "task": "Generate implementation code",
        "context": "{{architect_agent.result}}",
        "max_iterations": 8
      }
    }
  ]
}
```

---

## 🛠️ Configuration Options

### Environment Variables

```bash
# Jarvis Configuration  
JARVIS_KNOWLEDGE_PATH=./knowledge
JARVIS_MEMORY_ENABLED=true
JARVIS_MAX_AGENTS=10
JARVIS_DEFAULT_TIMEOUT=300

# Integration Settings
GHOSTFLOW_JARVIS_LOG_LEVEL=info
GHOSTFLOW_JARVIS_METRICS_ENABLED=true
```

### Agent Configuration

```rust
// Advanced agent configuration
let config = JarvisNodeConfig {
    default_agent_type: AgentType::General,
    max_execution_time: 600, // 10 minutes for complex tasks
    enable_memory: true,
    knowledge_base_path: Some("/data/knowledge".to_string()),
    
    // Advanced settings
    reasoning_depth: 5,
    tool_timeout: 30,
    memory_retention_days: 30,
    enable_learning: true,
};
```

---

## 📊 Monitoring & Observability  

### Metrics Collection

```rust
// Add metrics to your Jarvis nodes
use tracing::{info, span, Level};

let span = span!(Level::INFO, "jarvis_execution", 
    agent_type = agent_type_str,
    task_id = %task.id
);

let _enter = span.enter();
info!(
    "Agent execution completed",
    duration_ms = execution_time.as_millis(),
    iterations = task_result.iterations_used,
    success = task_result.success,
    confidence = task_result.confidence
);
```

### Performance Tracking

```json
{
  "jarvis_metrics": {
    "agent_executions": 1247,
    "avg_execution_time_ms": 2340,
    "success_rate": 0.94,
    "reasoning_iterations_avg": 3.2,
    "tools_usage": {
      "web_search": 342,
      "file_operations": 156,
      "code_analysis": 89
    }
  }
}
```

---

## 🔐 Security Considerations

### Agent Permissions

```rust
// Configure agent permissions
let permissions = AgentPermissions {
    allow_file_access: false,
    allow_network_access: true,
    allowed_domains: vec!["api.example.com".to_string()],
    max_tool_calls: 10,
    sandbox_enabled: true,
};
```

### Input Validation

```rust
// Validate and sanitize inputs
fn sanitize_task_input(input: &str) -> Result<String, GhostFlowError> {
    // Remove potentially dangerous content
    let sanitized = input
        .replace("<script", "")
        .replace("javascript:", "");
    
    if sanitized.len() > 10000 {
        return Err(GhostFlowError::ValidationError {
            message: "Task input too long".to_string(),
        });
    }
    
    Ok(sanitized)
}
```

---

## 🚀 Deployment Guide

### Docker Setup

```dockerfile
FROM rust:1.75 AS builder

# Install Zig for GhostLLM
RUN wget https://ziglang.org/builds/zig-linux-x86_64-0.16.0-dev.164+bc7955306.tar.xz \
    && tar -xf zig-linux-x86_64-0.16.0-dev.164+bc7955306.tar.xz \
    && mv zig-linux-x86_64-0.16.0-dev.164+bc7955306 /usr/local/zig \
    && ln -s /usr/local/zig/zig /usr/local/bin/zig

WORKDIR /app
COPY . .

# Build with both GhostLLM and Jarvis
RUN cargo build --release --features "ghostllm,jarvis"

FROM ubuntu:22.04
RUN apt-get update && apt-get install -y ca-certificates

# Copy knowledge base and models
COPY --from=builder /app/knowledge /app/knowledge
COPY --from=builder /app/models /app/models
COPY --from=builder /app/target/release/ghostflow /usr/local/bin/

EXPOSE 3000 8080
ENV JARVIS_KNOWLEDGE_PATH=/app/knowledge
ENV GHOSTLLM_MODEL_PATH=/app/models/default.gguf

CMD ["/usr/local/bin/ghostflow", "server"]
```

### Production Configuration

```yaml
# ghostflow-jarvis.yml
jarvis:
  agents:
    max_concurrent: 5
    default_timeout: 300
    memory:
      enabled: true
      retention_days: 30
      max_size_mb: 500
  
  knowledge_base:
    path: "/data/knowledge"
    auto_update: true
    sources:
      - type: "web_crawl"
        urls: ["https://docs.rs", "https://github.com/rust-lang"]
      - type: "files"
        path: "/data/documents"

  tools:
    enabled: ["web_search", "file_ops", "code_analysis"]
    web_search:
      provider: "duckduckgo"
      max_results: 10
    file_ops:
      allowed_dirs: ["/tmp", "/data/workspace"]
      max_file_size: 10485760  # 10MB

ghostflow:
  ui:
    enable_jarvis_metrics: true
    show_reasoning_steps: true
  
  performance:
    max_workflow_duration: 1800  # 30 minutes
    enable_agent_caching: true
```

---

## 🧪 Testing Framework

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_jarvis_general_agent() {
        let node = JarvisAgentNode::new().await;
        let context = ExecutionContext {
            node_id: "test_jarvis".to_string(),
            flow_id: "test_flow".to_string(), 
            execution_id: "test_exec".to_string(),
            input: serde_json::json!({
                "task": "Summarize the benefits of Rust programming language",
                "agent_type": "general",
                "max_iterations": 3
            }),
            variables: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        let result = node.execute(context).await.unwrap();
        assert!(result["success"].as_bool().unwrap());
        assert!(result["result"].is_object());
    }
    
    #[tokio::test]
    async fn test_multi_agent_workflow() {
        // Test complex multi-agent workflow
        let researcher = JarvisAgentNode::new().await;
        let analyst = JarvisAgentNode::new().await;
        
        // Research phase
        let research_result = researcher.execute(create_research_context()).await.unwrap();
        
        // Analysis phase with research result as context
        let analysis_context = create_analysis_context(&research_result);
        let analysis_result = analyst.execute(analysis_context).await.unwrap();
        
        assert!(analysis_result["success"].as_bool().unwrap());
        assert!(analysis_result["reasoning"].is_array());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_ghostflow_jarvis_pipeline() {
    let workflow = GhostFlowWorkflow::from_json(r#"
    {
        "nodes": [
            {
                "id": "jarvis_planner", 
                "type": "jarvis_agent",
                "config": {
                    "agent_type": "general",
                    "task": "Plan a blog post about AI workflows"
                }
            },
            {
                "id": "ghostllm_writer",
                "type": "ghostllm_generate", 
                "config": {
                    "prompt": "{{jarvis_planner.result.plan}}",
                    "max_tokens": 500
                }
            }
        ]
    }
    "#).unwrap();
    
    let result = workflow.execute().await.unwrap();
    assert!(result.is_success());
}
```

---

## 📈 Performance Optimization

### Agent Pooling

```rust
// Use agent pools for better performance
pub struct AgentPool {
    agents: Arc<RwLock<Vec<Arc<Agent>>>>,
    config: PoolConfig,
}

impl AgentPool {
    pub async fn get_agent(&self, agent_type: AgentType) -> Result<Arc<Agent>> {
        // Get available agent from pool or create new one
        let mut agents = self.agents.write().await;
        
        if let Some(agent) = agents.iter().find(|a| a.agent_type() == agent_type && a.is_idle()) {
            return Ok(agent.clone());
        }
        
        // Create new agent if pool not full
        if agents.len() < self.config.max_agents {
            let agent = Arc::new(Agent::new(agent_type).await?);
            agents.push(agent.clone());
            return Ok(agent);
        }
        
        // Wait for agent to become available
        self.wait_for_available_agent(agent_type).await
    }
}
```

### Caching Strategies

```rust
// Cache agent responses for similar tasks
#[derive(Clone)]
pub struct AgentCache {
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
}

impl AgentCache {
    pub async fn get_or_execute<F, Fut>(&self, key: &str, executor: F) -> Result<TaskResult>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<TaskResult>>,
    {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(key) {
                if !cached.is_expired() {
                    return Ok(cached.result.clone());
                }
            }
        }
        
        // Execute and cache result
        let result = executor().await?;
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), CachedResult::new(result.clone()));
        
        Ok(result)
    }
}
```

---

## 📚 Example Use Cases

### 1. **Automated Code Review**
- **Research Agent**: Gather coding standards and best practices
- **Code Agent**: Analyze submitted code
- **Writer Agent**: Generate detailed review comments

### 2. **Content Marketing Pipeline** 
- **Research Agent**: Find trending topics in target domain
- **Analyst Agent**: Analyze audience preferences and competitor content
- **Writer Agent**: Create engaging content with SEO optimization

### 3. **Data Analysis Workflows**
- **Analyst Agent**: Process raw data and identify patterns
- **General Agent**: Generate insights and recommendations  
- **Writer Agent**: Create executive summaries and reports

### 4. **Customer Support Automation**
- **General Agent**: Classify and route customer inquiries
- **Research Agent**: Find relevant documentation and solutions
- **Writer Agent**: Compose personalized responses

---

## 🔗 Additional Resources

- **Jarvis Repository**: https://github.com/ghostkellz/jarvis
- **GhostFlow Repository**: https://github.com/ghostkellz/ghostflow  
- **GhostLLM Integration**: `GHOSTLLM_INTEGRATION.md`
- **Zig FFI Guide**: `ZIG_INTEGRATION.md`

## 🤝 Contributing

1. Fork both repositories
2. Create feature branch: `git checkout -b feature/jarvis-integration`  
3. Add integration code and tests
4. Update documentation
5. Submit pull requests to both repos

## 📄 License

This integration guide follows the same license as both projects:
- **GhostFlow**: MIT License
- **Jarvis**: MIT License

---

**Ready to build intelligent automated workflows? Let's combine the power of GhostFlow's workflow engine with Jarvis AI agents! 🚀**