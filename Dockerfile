# OMEN Dockerfile
# Runtime-only image using pre-built binary

FROM archlinux:latest

# Create omen user (skip package install due to DNS issues)
RUN useradd -r -s /usr/bin/nologin -m -d /app omen 2>/dev/null || true

# Create necessary directories
RUN mkdir -p /app/data && chown -R omen:omen /app

# Copy pre-built binary from host
COPY target/x86_64-unknown-linux-gnu/release/omen /usr/local/bin/omen

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

# Health check (removed curl dependency)
# HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
#     CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["omen", "serve", "--config", "/app/omen.toml"]