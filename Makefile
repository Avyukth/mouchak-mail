# MCP Agent Mail - Makefile
# Unified build and run commands for the Rust implementation

.PHONY: all build build-release build-web build-web-leptos build-web-svelte dev run run-all clean test test-fast coverage audit quality-gate help export-static-data build-web-static deploy-github-pages full-deploy-github-pages push-github-pages

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

## Build web UI for production (SvelteKit - default)
build-web: build-web-svelte

## Build SvelteKit frontend
build-web-svelte:
	@echo "🌐 Building SvelteKit frontend..."
	cd crates/services/web-ui && bun install && bun run build
	@echo "✅ SvelteKit build complete (web-ui/build)"

## Build Leptos frontend (legacy)
build-web-leptos:
	@echo "🌐 Building Leptos frontend..."
	cd crates/services/web-ui-leptos && trunk build --release
	@echo "✅ Leptos build complete (web-ui-leptos/dist)"

## Build everything for production
build-prod: build-release build-web
	@echo "🎉 Production build complete"

# ============================================================================
# Development Commands
# ============================================================================

## Frontend dev with HMR (fast iteration - use with dev-backend)
## Usage: Terminal 1: make dev-backend | Terminal 2: make dev-frontend
dev-frontend:
	@echo "🌐 Starting SvelteKit dev server with HMR..."
	@echo "   Frontend: http://localhost:5173 (hot reload)"
	@echo "   API proxy: → http://localhost:9765/api/*"
	cd crates/services/web-ui && bun run dev

## Backend API server for frontend development
dev-backend:
	@echo "🚀 Starting backend API server..."
	@echo "   API: http://localhost:9765"
	@echo "   Tip: Use 'make dev-frontend' in another terminal for HMR"
	am serve http --port 9765

## Run API server (development)
dev-api:
	@echo "🚀 Starting API server on http://localhost:8000..."
	cargo run -p mcp-server

## Run server with embedded web UI on :8765 (SvelteKit - active)
dev-web:
	@echo "🌐 Building SvelteKit UI..."
	cd crates/services/web-ui && bun install && bun run build
	@echo "🚀 Starting server with embedded UI on http://localhost:8765..."
	cargo run -p mcp-agent-mail --features with-web-ui -- serve http --port 8765

## Run server with Leptos UI (requires changing embedded.rs to point to web-ui-leptos/dist)
dev-web-leptos:
	@echo "🌐 Building Leptos UI..."
	cd crates/services/web-ui-leptos && trunk build --release
	@echo "🚀 Starting server with embedded UI on http://localhost:8765..."
	cargo run -p mcp-agent-mail --features with-web-ui -- serve http --port 8765

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

## Build unified sidecar binary with embedded UI (SvelteKit - active)
build-sidecar:
	@echo "🔧 Building SvelteKit UI..."
	cd crates/services/web-ui && bun install && bun run build
	@echo "🔧 Building sidecar binary with embedded UI..."
	cargo build -p mcp-agent-mail --release --features with-web-ui
	@echo "✅ Binary: target/release/mcp-agent-mail"
	@ls -lh target/release/mcp-agent-mail

## Build sidecar with Leptos UI (requires changing embedded.rs)
build-sidecar-leptos:
	@echo "🔧 Building Leptos UI..."
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

## Install 'am' binary to ~/.local/bin
install-am: build-sidecar-minimal
	@echo "📦 Installing 'am' to ~/.local/bin..."
	@mkdir -p ~/.local/bin
	@cp target/release/mcp-agent-mail ~/.local/bin/am
	@chmod +x ~/.local/bin/am
	@echo "✅ Installed: ~/.local/bin/am"
	@echo "   Make sure ~/.local/bin is in your PATH"

## Install 'am' binary with embedded web UI
install-am-full: build-sidecar
	@echo "📦 Installing 'am' (with UI) to ~/.local/bin..."
	@mkdir -p ~/.local/bin
	@cp target/release/mcp-agent-mail ~/.local/bin/am
	@chmod +x ~/.local/bin/am
	@echo "✅ Installed: ~/.local/bin/am (with embedded UI)"
	@echo "   Make sure ~/.local/bin is in your PATH"

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
# GitHub Pages Static Deployment
# ============================================================================

## Export database to static JSON files for GitHub Pages
export-static-data:
	@echo "📤 Exporting data to static JSON..."
	@mkdir -p crates/services/web-ui/static/data
	./target/release/mcp-agent-mail share export static-data \
		--output crates/services/web-ui/static/data
	@echo "✅ Data exported to crates/services/web-ui/static/data/"

## Build SvelteKit for static GitHub Pages deployment
build-web-static: export-static-data
	@echo "🌐 Building SvelteKit for static deployment..."
	cd crates/services/web-ui && \
		VITE_DATA_MODE=static VITE_BUILD_MODE=static bun run build
	@echo "✅ Static build complete (web-ui/build-static)"

## Push static build to GitHub Pages (git-based, single commit)
## Env vars: GITHUB_PAGES_URL (repo URL), GITHUB_PAGES_CNAME (custom domain)
## Example: make push-github-pages GITHUB_PAGES_URL=https://github.com/user/repo.git GITHUB_PAGES_CNAME=mail.example.com
push-github-pages: build-web-static
	@echo "🚀 Deploying to GitHub Pages..."
	@if [ -z "$(GITHUB_PAGES_URL)" ]; then \
		echo "❌ Error: GITHUB_PAGES_URL is required"; \
		echo "   Example: make push-github-pages GITHUB_PAGES_URL=https://github.com/user/repo.git"; \
		exit 1; \
	fi
	cd crates/services/web-ui/build-static && \
		rm -rf .git && \
		git init && \
		git remote add origin $(GITHUB_PAGES_URL) && \
		$(if $(GITHUB_PAGES_CNAME),echo "$(GITHUB_PAGES_CNAME)" > CNAME &&) \
		git checkout -b gh-pages && \
		git add -A && \
		git commit -m "Deploy static site $$(date +%Y-%m-%d)" && \
		git push -f origin gh-pages
	@echo "✅ Deployed to GitHub Pages"
	@if [ -n "$(GITHUB_PAGES_CNAME)" ]; then \
		echo "🌐 Live at: https://$(GITHUB_PAGES_CNAME)/"; \
	fi

## Deploy to GitHub Pages (requires GITHUB_TOKEN)
## Usage: make deploy-github-pages GITHUB_PAGES_REPO=mail-archive
deploy-github-pages: build-web-static
	@echo "🚀 Deploying to GitHub Pages..."
	@if [ -z "$(GITHUB_PAGES_REPO)" ]; then \
		echo "❌ Error: GITHUB_PAGES_REPO is required"; \
		echo "   Usage: make deploy-github-pages GITHUB_PAGES_REPO=your-repo"; \
		exit 1; \
	fi
	./target/release/mcp-agent-mail share deploy github-pages \
		--build-dir crates/services/web-ui/build-static \
		--repo $(GITHUB_PAGES_REPO) \
		$(if $(GITHUB_PAGES_DOMAIN),--custom-domain $(GITHUB_PAGES_DOMAIN))
	@echo "✅ Deployment complete!"

## Full GitHub Pages workflow: build release + export + build static + deploy
full-deploy-github-pages: build-release build-web-static deploy-github-pages
	@echo "🎉 Full GitHub Pages deployment complete!"

# ============================================================================
# Git Hooks
# ============================================================================

## Install git hooks (via prek)
install-hooks:
	@echo "Installing git hooks via prek..."
	@prek install
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
	rm -rf crates/services/web-ui/build
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
	@echo "Development:"
	@echo "  dev          Run API + Web UI in parallel"
	@echo "  dev-api      Run API server only"
	@echo "  dev-web      Build SvelteKit UI and serve on :8765"
	@echo "  dev-web-leptos  Build Leptos UI and serve on :8765"
	@echo "  dev-mcp      Run MCP stdio server"
	@echo ""
	@echo "Production:"
	@echo "  run          Run API server (release mode)"
	@echo "  run-prod     Build and run production server"
	@echo ""
	@echo "Sidecar (single binary):"
	@echo "  build-sidecar         Build binary with embedded SvelteKit UI"
	@echo "  build-sidecar-leptos  Build binary with embedded Leptos UI"
	@echo "  build-sidecar-minimal Build minimal binary (no UI)"
	@echo "  build-claude-desktop  Build for Claude Desktop integration"
	@echo ""
	@echo "GitHub Pages Deployment:"
	@echo "  export-static-data      Export DB to static JSON files"
	@echo "  build-web-static        Build static site for GitHub Pages"
	@echo "  push-github-pages       Git push (GITHUB_PAGES_URL, GITHUB_PAGES_CNAME)"
	@echo "  deploy-github-pages     Deploy via API (needs GITHUB_PAGES_REPO)"
	@echo "  full-deploy-github-pages Complete workflow: build + export + deploy"
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
