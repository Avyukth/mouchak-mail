# Mouchak Mail - Justfile
# Modern command runner (https://github.com/casey/just)
# Install: cargo install just

# Default recipe
default: dev

# ============================================================================
# Development
# ============================================================================

# Run all dev servers (API + Web UI)
dev:
    @echo "🚀 Starting development servers..."
    @echo "   API:    http://localhost:8000"
    @echo "   Web UI: http://localhost:5173"
    ./scripts/dev.sh

# Run API server only
api:
    cargo run -p mcp-server

# Run web UI only (hot reload)
web:
    cd crates/services/web-ui && bun run dev

# Run MCP stdio server
mcp:
    cargo run -p mcp-stdio -- serve

# ============================================================================
# Build
# ============================================================================

# Build debug
build:
    cargo build --workspace

# Build release
release:
    cargo build --workspace --release

# Build web UI for production
build-web:
    cd crates/services/web-ui && bun install && bun run build

# Build everything for production
prod: release build-web
    @echo "✅ Production build complete"

# ============================================================================
# Testing & Quality
# ============================================================================

# Run all tests
test:
    cargo test -p lib-core --test integration -- --test-threads=1
    cargo test -p mcp-stdio --test integration -- --test-threads=1

# Run clippy
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check format
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

# ============================================================================
# Mutation Testing
# ============================================================================

# Install cargo-mutants
install-mutants:
    cargo install cargo-mutants

# Run mutation testing on lib-mcp
mutate:
    cargo mutants --package lib-mcp

# Run mutation testing (verbose)
mutate-verbose:
    cargo mutants --package lib-mcp -vV

# Run mutation testing on lib-core
mutate-core:
    cargo mutants --package lib-core

# ============================================================================
# Utilities
# ============================================================================

# List MCP tools
tools:
    cargo run -p mcp-stdio -- tools

# Export schema
schema:
    cargo run -p mcp-stdio -- schema

# Clean everything
clean:
    cargo clean
    rm -rf crates/services/web-ui/.svelte-kit crates/services/web-ui/build

# Show ready work
ready:
    bd ready --json

# Show help
help:
    @just --list
