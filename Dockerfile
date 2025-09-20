# OMEN Dockerfile
# Multi-stage build for optimized production image

# Build stage
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy dependency files
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src target/release/deps/omen*

# Copy actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create omen user
RUN useradd -r -s /bin/false -m -d /app omen

# Create necessary directories
RUN mkdir -p /app/data && chown -R omen:omen /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/omen /usr/local/bin/omen

# Copy configuration files
COPY omen.toml /app/omen.toml
COPY .env.example /app/.env.example

# Set ownership
RUN chown -R omen:omen /app

# Switch to non-root user
USER omen
WORKDIR /app

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["omen", "serve", "--config", "/app/omen.toml"]