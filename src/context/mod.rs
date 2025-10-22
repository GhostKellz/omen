//! Context management for OMEN
//!
//! Provides persistent workspace memory, session history, and project-aware
//! context loading for AI requests.

pub mod workspace;
pub mod session;
pub mod embeddings;

#[allow(unused_imports)]
pub use workspace::{Workspace, WorkspaceConfig};
#[allow(unused_imports)]
pub use session::{SessionHistory, Session, Query};
#[allow(unused_imports)]
pub use embeddings::{EmbeddingsStore, CodeChunk};

use crate::error::Result;

/// Context manager - coordinates workspace, sessions, and embeddings
/// Planned feature for project-aware AI routing
#[allow(dead_code)]
pub struct ContextManager {
    workspace: Option<Workspace>,
}

#[allow(dead_code)]
impl ContextManager {
    /// Create new context manager
    pub fn new() -> Self {
        Self {
            workspace: None,
        }
    }

    /// Load workspace from path (auto-detects .omen/)
    pub async fn load_workspace(&mut self, project_root: &str) -> Result<()> {
        let workspace = Workspace::from_path(project_root).await?;
        self.workspace = Some(workspace);
        Ok(())
    }

    /// Get current workspace
    pub fn workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }

    /// Build context for AI request
    pub async fn build_context(&self, prompt: &str) -> Result<String> {
        if let Some(workspace) = &self.workspace {
            workspace.build_context(prompt).await
        } else {
            Ok(String::new())
        }
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}
