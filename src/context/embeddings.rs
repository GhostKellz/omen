//! Embeddings store - vector memory for code search

use crate::error::{OmenError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::{SqlitePoolOptions, SqliteConnectOptions}, Row};
use std::path::Path;
use std::str::FromStr;

/// Code chunk with embedding
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: i64,
    pub file_path: String,
    pub chunk_index: usize,
    pub content: String,
    pub language: String,
    pub git_commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,  // 768-dim vector from nomic-embed-text
    pub last_updated: String,
}

/// Embeddings store with SQLite backend
/// Planned feature for vector search over code
#[allow(dead_code)]
pub struct EmbeddingsStore {
    pool: SqlitePool,
    ollama_endpoint: String,
}

#[allow(dead_code)]
impl EmbeddingsStore {
    /// Open or create embeddings store
    pub async fn open<P: AsRef<Path>>(db_path: P, ollama_endpoint: &str) -> Result<Self> {
        let path = db_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| OmenError::Database(format!("Failed to create parent directory: {}", e)))?;
        }

        // Use absolute path
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| OmenError::Database(format!("Failed to get current dir: {}", e)))?
                .join(path)
        };

        // Create connect options with create_if_missing
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", absolute_path.display()))
            .map_err(|e| OmenError::Database(format!("Failed to parse database URL: {}", e)))?
            .create_if_missing(true);

        // Create pool with proper options
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| OmenError::Database(format!("Failed to connect to embeddings DB: {}", e)))?;

        // Run migrations
        Self::migrate(&pool).await?;

        Ok(Self {
            pool,
            ollama_endpoint: ollama_endpoint.to_string(),
        })
    }

    /// Run database migrations
    async fn migrate(pool: &SqlitePool) -> Result<()> {
        // Create embeddings table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS embeddings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                language TEXT NOT NULL,
                git_commit TEXT NOT NULL,
                embedding BLOB,
                last_updated TEXT NOT NULL,
                UNIQUE(file_path, chunk_index)
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to create embeddings table: {}", e)))?;

        // Create index for fast lookup
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_file_path ON embeddings(file_path)")
            .execute(pool)
            .await
            .ok();

        Ok(())
    }

    /// Generate embedding for text using Ollama
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let client = reqwest::Client::new();

        #[derive(Serialize)]
        struct EmbedRequest {
            model: String,
            prompt: String,
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            embedding: Vec<f32>,
        }

        let response = client
            .post(format!("{}/api/embeddings", self.ollama_endpoint))
            .json(&EmbedRequest {
                model: "nomic-embed-text".to_string(),
                prompt: text.to_string(),
            })
            .send()
            .await
            .map_err(|e| OmenError::Provider(format!("Failed to generate embedding: {}", e)))?;

        let embed_resp: EmbedResponse = response
            .json()
            .await
            .map_err(|e| OmenError::Provider(format!("Failed to parse embedding response: {}", e)))?;

        Ok(embed_resp.embedding)
    }

    /// Index a code chunk with embedding
    pub async fn index_chunk(&self, mut chunk: CodeChunk) -> Result<()> {
        // Generate embedding if not present
        if chunk.embedding.is_none() {
            let embedding = self.generate_embedding(&chunk.content).await?;
            chunk.embedding = Some(embedding);
        }

        // Serialize embedding to binary
        let embedding_bytes = bincode::serialize(&chunk.embedding)
            .map_err(|e| OmenError::Database(format!("Failed to serialize embedding: {}", e)))?;

        // Insert or replace
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO embeddings
            (file_path, chunk_index, content, language, git_commit, embedding, last_updated)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&chunk.file_path)
        .bind(chunk.chunk_index as i64)
        .bind(&chunk.content)
        .bind(&chunk.language)
        .bind(&chunk.git_commit)
        .bind(&embedding_bytes)
        .bind(&chunk.last_updated)
        .execute(&self.pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to index chunk: {}", e)))?;

        Ok(())
    }

    /// Search for similar code chunks (simple cosine similarity)
    pub async fn search_similar(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<CodeChunk>> {
        // Get all embeddings
        let rows = sqlx::query(
            r#"
            SELECT id, file_path, chunk_index, content, language, git_commit, embedding, last_updated
            FROM embeddings
            WHERE embedding IS NOT NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to fetch embeddings: {}", e)))?;

        let mut results: Vec<(f32, CodeChunk)> = Vec::new();

        for row in rows {
            let id: i64 = row.get("id");
            let file_path: String = row.get("file_path");
            let chunk_index: i64 = row.get("chunk_index");
            let content: String = row.get("content");
            let language: String = row.get("language");
            let git_commit: String = row.get("git_commit");
            let embedding_bytes: Vec<u8> = row.get("embedding");
            let last_updated: String = row.get("last_updated");

            // Deserialize embedding
            let embedding_opt: Option<Vec<f32>> = bincode::deserialize(&embedding_bytes)
                .map_err(|e| OmenError::Database(format!("Failed to deserialize embedding: {}", e)))?;

            if let Some(embedding) = embedding_opt {
                // Calculate cosine similarity
                let similarity = Self::cosine_similarity(query_embedding, &embedding);

                results.push((
                    similarity,
                    CodeChunk {
                        id,
                        file_path,
                        chunk_index: chunk_index as usize,
                        content,
                        language,
                        git_commit,
                        embedding: Some(embedding),
                        last_updated,
                    },
                ));
            }
        }

        // Sort by similarity (highest first)
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Return top-k results
        Ok(results.into_iter().take(top_k).map(|(_, chunk)| chunk).collect())
    }

    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    /// Clear all embeddings
    pub async fn clear(&self) -> Result<()> {
        sqlx::query("DELETE FROM embeddings")
            .execute(&self.pool)
            .await
            .map_err(|e| OmenError::Database(format!("Failed to clear embeddings: {}", e)))?;

        Ok(())
    }

    /// Get total number of chunks indexed
    pub async fn count(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM embeddings")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| OmenError::Database(format!("Failed to count embeddings: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count as usize)
    }
}
