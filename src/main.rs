use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod billing;
mod config;
mod error;
mod ghost_ai;
// TODO: Re-enable after fixing compilation issues
// mod grpc;
mod multiplexer;
mod providers;
mod rate_limiter;
mod router;
mod routing;
mod server;
mod types;

use config::Config;
use server::Server;

#[derive(Parser)]
#[command(name = "omen")]
#[command(about = "OMEN - Open Model Exchange Network")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the OMEN server
    Serve {
        /// Configuration file path
        #[arg(short, long, default_value = "omen.toml")]
        config: String,
        /// Bind address
        #[arg(short, long)]
        bind: Option<String>,
        /// Port to listen on
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Check configuration and provider health
    Check {
        /// Configuration file path
        #[arg(short, long, default_value = "omen.toml")]
        config: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "omen=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config, bind, port } => {
            info!("🚀 Starting OMEN v{}", env!("CARGO_PKG_VERSION"));

            // Load configuration
            let mut cfg = Config::load(&config).await?;

            // Override with CLI arguments
            if let Some(bind_addr) = bind {
                cfg.server.bind = bind_addr;
            }
            if let Some(port_num) = port {
                cfg.server.port = port_num;
            }

            // Start server
            let server = Server::new(cfg).await?;
            server.start().await?;
        }
        Commands::Check { config } => {
            info!("🔍 Checking OMEN configuration...");

            let _cfg = Config::load(&config).await?;
            info!("✅ Configuration loaded successfully");

            // TODO: Check provider health
            warn!("⚠️  Provider health checks not yet implemented");

            info!("✅ Configuration check complete");
        }
    }

    Ok(())
}
