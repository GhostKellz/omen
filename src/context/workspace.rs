//! Workspace management - persistent project context

use crate::error::{OmenError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Workspace - represents a project with persistent context
/// Planned feature for project-aware AI routing
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub config: WorkspaceConfig,
}

/// Workspace configuration (stored in .omen/workspace.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub project: ProjectInfo,
    pub ai: AIConfig,
    #[serde(default)]
    pub dependencies: Dependencies,
    #[serde(default)]
    pub git: GitContext,
    #[serde(default)]
    pub embeddings: EmbeddingsInfo,
    #[serde(default)]
    pub environment: EnvironmentConfig,
    #[serde(default)]
    pub usage: UsageStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub project_type: String,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub last_model: String,
    pub context_profile: Vec<String>,
    pub preferred_provider: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust: Option<RustDeps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonDeps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<NodeDeps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker: Option<DockerDeps>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustDeps {
    pub toolchain: String,
    pub cargo_toml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonDeps {
    pub version: String,
    pub requirements: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDeps {
    pub version: String,
    pub package_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerDeps {
    pub compose: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitContext {
    pub last_commit: String,
    pub current_branch: String,
    pub recent_changes: RecentChanges,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentChanges {
    pub files_modified: usize,
    pub focus_areas: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbeddingsInfo {
    pub last_indexed: String,
    pub total_chunks: usize,
    pub model: String,
    pub index_file: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub auto_activate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub venv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zig_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_toolchain: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_queries: usize,
    pub cost_this_month: f64,
    pub preferred_tasks: Vec<String>,
}

#[allow(dead_code)]
impl Workspace {
    /// Load or create workspace from project root
    pub async fn from_path<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let root = project_root.as_ref().to_path_buf();
        let omen_dir = root.join(".omen");

        // Check if .omen/ exists
        if !omen_dir.exists() {
            tracing::info!("Creating new workspace at {}", omen_dir.display());
            fs::create_dir_all(&omen_dir).await.map_err(|e| {
                OmenError::Config(format!("Failed to create .omen/ directory: {}", e))
            })?;

            // Initialize new workspace
            let config = Self::init_workspace(&root, &omen_dir).await?;
            return Ok(Self { root, config });
        }

        // Load existing workspace
        let config_file = omen_dir.join("workspace.yaml");
        if !config_file.exists() {
            // .omen/ exists but no config - initialize
            let config = Self::init_workspace(&root, &omen_dir).await?;
            return Ok(Self { root, config });
        }

        // Read and parse config
        let content = fs::read_to_string(&config_file).await.map_err(|e| {
            OmenError::Config(format!("Failed to read workspace config: {}", e))
        })?;

        let config: WorkspaceConfig = serde_yaml::from_str(&content).map_err(|e| {
            OmenError::Config(format!("Failed to parse workspace config: {}", e))
        })?;

        tracing::info!("Loaded workspace: {}", config.project.name);

        Ok(Self { root, config })
    }

    /// Initialize new workspace
    async fn init_workspace(root: &Path, omen_dir: &Path) -> Result<WorkspaceConfig> {
        // Detect project type
        let project_type = Self::detect_project_type(root).await;
        let project_name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Detect git info
        let git = Self::detect_git_context(root).await;

        // Create default config
        let config = WorkspaceConfig {
            project: ProjectInfo {
                name: project_name,
                project_type: project_type.clone(),
                root: root.to_string_lossy().to_string(),
            },
            ai: AIConfig {
                last_model: "qwen2.5-coder-32b".to_string(),
                context_profile: vec!["coding".to_string(), project_type],
                preferred_provider: "ollama".to_string(),
            },
            dependencies: Self::detect_dependencies(root).await,
            git,
            embeddings: EmbeddingsInfo {
                last_indexed: chrono::Utc::now().to_rfc3339(),
                total_chunks: 0,
                model: "nomic-embed-text".to_string(),
                index_file: "embeddings.db".to_string(),
            },
            environment: EnvironmentConfig {
                auto_activate: true,
                venv: None,
                node_version: None,
                zig_version: None,
                rust_toolchain: Some("stable".to_string()),
            },
            usage: UsageStats::default(),
        };

        // Write config
        let config_file = omen_dir.join("workspace.yaml");
        let yaml = serde_yaml::to_string(&config).map_err(|e| {
            OmenError::Config(format!("Failed to serialize workspace config: {}", e))
        })?;

        fs::write(&config_file, yaml).await.map_err(|e| {
            OmenError::Config(format!("Failed to write workspace config: {}", e))
        })?;

        tracing::info!("Initialized workspace: {}", config.project.name);

        Ok(config)
    }

    /// Detect project type from files
    async fn detect_project_type(root: &Path) -> String {
        if root.join("Cargo.toml").exists() {
            "rust".to_string()
        } else if root.join("package.json").exists() {
            "node".to_string()
        } else if root.join("requirements.txt").exists() || root.join("pyproject.toml").exists() {
            "python".to_string()
        } else if root.join("build.zig").exists() {
            "zig".to_string()
        } else if root.join("go.mod").exists() {
            "go".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Detect dependencies
    async fn detect_dependencies(root: &Path) -> Dependencies {
        let mut deps = Dependencies::default();

        if root.join("Cargo.toml").exists() {
            deps.rust = Some(RustDeps {
                toolchain: "stable".to_string(),
                cargo_toml: "Cargo.toml".to_string(),
            });
        }

        if root.join("package.json").exists() {
            deps.node = Some(NodeDeps {
                version: "18.0.0".to_string(),
                package_json: "package.json".to_string(),
            });
        }

        if root.join("requirements.txt").exists() {
            deps.python = Some(PythonDeps {
                version: "3.11".to_string(),
                requirements: "requirements.txt".to_string(),
            });
        }

        if root.join("docker-compose.yml").exists() {
            deps.docker = Some(DockerDeps {
                compose: "docker-compose.yml".to_string(),
            });
        }

        deps
    }

    /// Detect git context
    async fn detect_git_context(root: &Path) -> GitContext {
        // Try to get git info using tokio::process::Command
        let branch = tokio::process::Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .current_dir(root)
            .output()
            .await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "main".to_string());

        let commit = tokio::process::Command::new("git")
            .arg("rev-parse")
            .arg("--short")
            .arg("HEAD")
            .current_dir(root)
            .output()
            .await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        GitContext {
            last_commit: commit,
            current_branch: branch,
            recent_changes: RecentChanges::default(),
        }
    }

    /// Build context string for AI request
    pub async fn build_context(&self, prompt: &str) -> Result<String> {
        let mut context = String::new();

        // Add project info
        context.push_str(&format!("Project: {} ({})\n",
            self.config.project.name,
            self.config.project.project_type
        ));

        // Add git context
        context.push_str(&format!("Branch: {}\n", self.config.git.current_branch));
        context.push_str(&format!("Last commit: {}\n", self.config.git.last_commit));

        // Add recent changes if available
        if self.config.git.recent_changes.files_modified > 0 {
            context.push_str(&format!("\nRecent changes: {} files modified\n",
                self.config.git.recent_changes.files_modified
            ));
            if !self.config.git.recent_changes.focus_areas.is_empty() {
                context.push_str(&format!("Focus areas: {}\n",
                    self.config.git.recent_changes.focus_areas.join(", ")
                ));
            }
        }

        // Add semantic search results from embeddings if available
        if self.config.embeddings.total_chunks > 0 && !prompt.is_empty() {
            // Try to search embeddings (use default ollama endpoint)
            match self.search_embeddings(prompt, "http://localhost:11434").await {
                Ok(chunks) if !chunks.is_empty() => {
                    context.push_str("\n=== Relevant Code Context ===\n");
                    for (idx, chunk) in chunks.iter().enumerate() {
                        context.push_str(&format!("\n[{}] {}: Line {}\n",
                            idx + 1,
                            chunk.file_path,
                            chunk.chunk_index
                        ));
                        context.push_str(&format!("```{}\n{}\n```\n",
                            chunk.language,
                            chunk.content
                        ));
                    }
                }
                _ => {
                    // Embeddings search failed or returned no results - that's ok
                    tracing::debug!("No embedding results for query");
                }
            }
        }

        Ok(context)
    }

    /// Search embeddings for relevant code (requires embeddings to be indexed first)
    pub async fn search_embeddings(&self, query: &str, ollama_endpoint: &str) -> Result<Vec<super::CodeChunk>> {
        use super::EmbeddingsStore;

        let embeddings_db = self.root.join(".omen/embeddings.db");

        // Open embeddings store
        let store = EmbeddingsStore::open(&embeddings_db, ollama_endpoint).await?;

        // Generate embedding for query
        let query_embedding = store.generate_embedding(query).await?;

        // Search for similar chunks
        let results = store.search_similar(&query_embedding, 5).await?;

        Ok(results)
    }

    /// Save workspace config
    pub async fn save(&self) -> Result<()> {
        let config_file = self.root.join(".omen/workspace.yaml");
        let yaml = serde_yaml::to_string(&self.config).map_err(|e| {
            OmenError::Config(format!("Failed to serialize workspace config: {}", e))
        })?;

        fs::write(&config_file, yaml).await.map_err(|e| {
            OmenError::Config(format!("Failed to write workspace config: {}", e))
        })?;

        Ok(())
    }
}
