// Test embedding functionality

use omen::context::{CodeChunk, EmbeddingsStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Testing Omen Embedding System\n");

    // Use current directory for test
    let db_path = "/tmp/test_embeddings.db";
    let ollama_endpoint = "http://localhost:11434";

    // Initialize store
    println!("📦 Initializing embeddings store...");
    let store = EmbeddingsStore::open(&db_path, ollama_endpoint).await?;

    // Create some test code chunks
    let chunks = vec![
        CodeChunk {
            id: 0,
            file_path: "src/auth/login.rs".to_string(),
            chunk_index: 0,
            content: r#"
pub async fn authenticate_user(username: &str, password: &str) -> Result<User> {
    let user = database::find_user_by_username(username).await?;
    if verify_password(password, &user.password_hash) {
        Ok(user)
    } else {
        Err(AuthError::InvalidCredentials)
    }
}
"#.to_string(),
            language: "rust".to_string(),
            git_commit: "abc123".to_string(),
            embedding: None,
            last_updated: chrono::Utc::now().to_rfc3339(),
        },
        CodeChunk {
            id: 0,
            file_path: "src/database/connection.rs".to_string(),
            chunk_index: 0,
            content: r#"
pub async fn connect_to_database(config: &DbConfig) -> Result<Pool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(&config.url)
        .await?;
    Ok(pool)
}
"#.to_string(),
            language: "rust".to_string(),
            git_commit: "abc123".to_string(),
            embedding: None,
            last_updated: chrono::Utc::now().to_rfc3339(),
        },
        CodeChunk {
            id: 0,
            file_path: "src/api/routes.rs".to_string(),
            chunk_index: 0,
            content: r#"
pub async fn handle_login(req: LoginRequest) -> Response {
    let user = authenticate_user(&req.username, &req.password).await?;
    let token = generate_jwt_token(&user)?;
    Ok(Json(LoginResponse { token }))
}
"#.to_string(),
            language: "rust".to_string(),
            git_commit: "abc123".to_string(),
            embedding: None,
            last_updated: chrono::Utc::now().to_rfc3339(),
        },
    ];

    // Index chunks
    println!("🔄 Indexing {} code chunks...", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        print!("  [{}/{}] Indexing {}...", i + 1, chunks.len(), chunk.file_path);
        store.index_chunk(chunk.clone()).await?;
        println!(" ✓");
    }

    let count = store.count().await?;
    println!("\n✅ Indexed {} chunks total\n", count);

    // Test search
    let queries = vec![
        "how to authenticate a user",
        "database connection",
        "JWT token generation",
    ];

    for query in queries {
        println!("🔎 Query: \"{}\"", query);
        let results = store.search_similar(
            &store.generate_embedding(query).await?,
            3
        ).await?;

        for (i, chunk) in results.iter().enumerate() {
            println!("  {}. {} (chunk {})",
                i + 1,
                chunk.file_path,
                chunk.chunk_index
            );
            println!("     {}", chunk.content.lines().next().unwrap_or("").trim());
        }
        println!();
    }

    println!("✨ Embedding system working!");
    println!("Database: {}", db_path);

    Ok(())
}
