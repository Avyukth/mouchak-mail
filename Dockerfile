# =============================================================================
# MCP Agent Mail - Multi-stage Dockerfile
# =============================================================================
# Build stages:
#   1. chef    - Install cargo-chef for dependency caching
#   2. planner - Analyze dependencies and create recipe
#   3. builder - Build the actual binary with cached dependencies
#   4. runtime - Minimal runtime image
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Chef - Install cargo-chef
# -----------------------------------------------------------------------------
FROM rust:1.83-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# -----------------------------------------------------------------------------
# Stage 2: Planner - Analyze dependencies
# -----------------------------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# -----------------------------------------------------------------------------
# Stage 3: Builder - Build with cached dependencies
# -----------------------------------------------------------------------------
FROM chef AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy recipe and build dependencies (this layer is cached)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and build
COPY . .
RUN cargo build --release --bin mcp-server

# -----------------------------------------------------------------------------
# Stage 4: Runtime - Minimal image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false mcp

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/mcp-server /app/mcp-server

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

# Create data directories
RUN mkdir -p /app/data/archive && chown -R mcp:mcp /app

# Switch to non-root user
USER mcp

# Environment variables
ENV RUST_LOG=info
ENV LOG_FORMAT=json
ENV PORT=8000

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Run the server
CMD ["/app/mcp-server"]
