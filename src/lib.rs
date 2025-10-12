//! OMEN - Open Model Exchange Network
//!
//! A universal AI API gateway with smart routing, provider adapters,
//! usage controls, and Ghost Stack integration.
//!
//! This library provides the core functionality of OMEN that can be
//! integrated into other applications like GhostLLM.

pub mod auth;
pub mod billing;
pub mod cache;
pub mod config;
pub mod error;
pub mod ghost_ai;
pub mod grpc;
pub mod multiplexer;
pub mod providers;
pub mod rate_limiter;
pub mod router;
pub mod routing;
pub mod server;
pub mod types;

// Re-export commonly used types
pub use config::Config;
pub use error::{OmenError, Result};
pub use server::Server;
pub use types::*;

/// Initialize OMEN's tracing/logging subsystem
pub fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "omen=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
