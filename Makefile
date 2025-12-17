# MCP Agent Mail - Makefile
# Unified build and run commands for the Rust implementation

.PHONY: all build build-release build-web dev run run-all clean test test-fast coverage audit quality-gate help

# Default target
all: build

# ============================================================================
# Build Commands
# ============================================================================

## Build all components (debug)
build:
	@echo "🔨 Building Rust components..."
	cargo build --workspace
	@echo "✅ Build complete"

## Build all components (release)
build-release:
	@echo "🔨 Building Rust components (release)..."
	cargo build --workspace --release
	@echo "✅ Release build complete"

## Build web UI for production
build-web:
	@echo "🌐 Building Leptos frontend..."
	cd crates/services/web-ui-leptos && trunk build --release
	@echo "✅ Frontend build complete"

## Build everything for production
build-prod: build-release build-web
	@echo "🎉 Production build complete"

# ============================================================================
# Development Commands
# ============================================================================

## Run API server (development)
dev-api:
	@echo "🚀 Starting API server on http://localhost:8000..."
	cargo run -p mcp-server

## Run web UI (development with hot reload)
dev-web:
	@echo "🌐 Starting Leptos dev server on http://localhost:8080..."
	cd crates/services/web-ui-leptos && trunk serve --open

## Run MCP stdio server
dev-mcp:
	@echo "📨 Starting MCP stdio server..."
	cargo run -p mcp-stdio -- serve

## Run all services in development (parallel)
dev:
	@echo "🚀 Starting all development servers..."
	@echo "   API: http://localhost:8000"
	@echo "   Web: http://localhost:5173"
	$(MAKE) -j2 dev-api dev-web

# ============================================================================
# Production Commands
# ============================================================================

## Run API server (release mode)
run:
	@echo "🚀 Starting API server (release) on http://localhost:8000..."
	cargo run -p mcp-server --release

## Run integrated server with built frontend
run-prod: build-prod
	@echo "🚀 Starting production server on http://localhost:8000..."
	@echo "   API:    http://localhost:8000/api/*"
	@echo "   Web UI: http://localhost:8000/mail"
	cargo run -p mcp-server --release

# ============================================================================
# Sidecar Build Targets
# ============================================================================

## Build unified sidecar binary with embedded UI
build-sidecar:
	@echo "🔧 Building web UI..."
	cd crates/services/web-ui-leptos && trunk build --release
	@echo "🔧 Building sidecar binary with embedded UI..."
	cargo build -p mcp-agent-mail --release --features with-web-ui
	@echo "✅ Binary: target/release/mcp-agent-mail"
	@ls -lh target/release/mcp-agent-mail

## Build minimal sidecar binary (no UI)
build-sidecar-minimal:
	@echo "🔧 Building minimal sidecar binary (no UI)..."
	cargo build -p mcp-agent-mail --release
	@echo "✅ Minimal binary: target/release/mcp-agent-mail"
	@ls -lh target/release/mcp-agent-mail

## Build for Claude Desktop (stdio MCP server)
build-claude-desktop: build-sidecar-minimal
	@echo "📋 Add to claude_desktop_config.json:"
	@echo '  "agent-mail": { "command": "$(PWD)/target/release/mcp-agent-mail", "args": ["serve", "mcp", "--transport", "stdio"] }'

# ============================================================================
# Testing
# ============================================================================

## Run all tests
test:
	@echo "🧪 Running all tests..."
	cargo test -p lib-core --test integration -- --test-threads=1
	cargo test -p mcp-stdio --test integration -- --test-threads=1
	@echo "✅ All tests passed (26 total)"

## Run tests with coverage
test-coverage:
	@echo "🧪 Running tests with coverage..."
	cargo tarpaulin --workspace --out Html

## Run unit tests only (fast)
test-fast:
	@echo "🧪 Running unit tests (fast)..."
	cargo test --workspace --lib
	@echo "✅ Unit tests passed"

## Generate coverage report
coverage:
	@echo "📊 Generating coverage report..."
	cargo llvm-cov --workspace --html
	@echo "✅ Coverage report: target/llvm-cov/html/index.html"

## Run security audits
audit:
	@echo "🔒 Running security audits..."
	cargo audit
	cargo deny check
	@echo "✅ Security audits passed"

## Run all quality gates
quality-gate:
	@echo "🎯 Running quality gates..."
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test
	pmat analyze tdg --fail-on-violation
	@echo "✅ All quality gates passed"

## Run clippy lints
lint:
	@echo "🔍 Running clippy..."
	cargo clippy --workspace --all-targets -- -D warnings

## Format code
fmt:
	@echo "✨ Formatting code..."
	cargo fmt --all

## Check formatting without modifying
fmt-check:
	@echo "✨ Checking code format..."
	cargo fmt --all -- --check

# ============================================================================
# Database
# ============================================================================

## Initialize/reset database
db-reset:
	@echo "🗄️  Resetting database..."
	rm -f data/storage.db
	@echo "✅ Database reset (will be recreated on next run)"

## Show database path
db-info:
	@echo "📂 Database location: data/storage.db"
	@ls -lh data/storage.db 2>/dev/null || echo "   (not created yet)"

# ============================================================================
# Git Hooks
# ============================================================================

## Install git hooks (cargo-husky auto-installs on cargo test)
install-hooks:
	@echo "Installing git hooks via cargo-husky..."
	@cargo test --quiet 2>/dev/null || true
	@echo "Hooks installed at .git/hooks/"
	@ls -la .git/hooks/pre-commit

## Verify hook installation
check-hooks:
	@echo "Checking git hooks..."
	@if [ -f .git/hooks/pre-commit ]; then \
		echo "  pre-commit hook: installed"; \
	else \
		echo "  pre-commit hook: NOT installed (run 'make install-hooks')"; \
	fi

# ============================================================================
# Utilities
# ============================================================================

## List all MCP tools
tools:
	@cargo run -p mcp-stdio -- tools

## Export JSON schema for all tools
schema:
	@cargo run -p mcp-stdio -- schema

## Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	rm -rf crates/services/web-ui-leptos/dist
	@echo "✅ Clean complete"

## Check beads ready work
ready:
	@bd ready --json

## Show help
help:
	@echo "MCP Agent Mail - Rust Implementation"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Build:"
	@echo "  build        Build all Rust components (debug)"
	@echo "  build-release Build all Rust components (release)"
	@echo "  build-web    Build SvelteKit frontend for production"
	@echo "  build-prod   Build everything for production"
	@echo ""
	@echo "Development (with hot reload):"
	@echo "  dev          Run API + Web UI in parallel"
	@echo "  dev-api      Run API server only"
	@echo "  dev-web      Run SvelteKit dev server only"
	@echo "  dev-mcp      Run MCP stdio server"
	@echo ""
	@echo "Production:"
	@echo "  run          Run API server (release mode)"
	@echo "  run-prod     Build and run production server"
	@echo ""
	@echo "Sidecar (single binary):"
	@echo "  build-sidecar         Build binary with embedded web UI"
	@echo "  build-sidecar-minimal Build minimal binary (no UI)"
	@echo "  build-claude-desktop  Build for Claude Desktop integration"
	@echo ""
	@echo "Testing:"
	@echo "  test         Run all integration tests"
	@echo "  test-fast    Run unit tests only (fast)"
	@echo "  coverage     Generate coverage report"
	@echo "  audit        Run security audits"
	@echo "  lint         Run clippy lints"
	@echo "  fmt          Format code"
	@echo "  quality-gate Run all quality gates"
	@echo ""
	@echo "Git Hooks:"
	@echo "  install-hooks  Install pre-commit hooks via cargo-husky"
	@echo "  check-hooks    Verify hook installation"
	@echo ""
	@echo "Utilities:"
	@echo "  tools        List all MCP tools"
	@echo "  schema       Export JSON schema for tools"
	@echo "  clean        Remove build artifacts"
	@echo "  ready        Show beads ready work"
	@echo "  help         Show this help message"
