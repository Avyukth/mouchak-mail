# Single Binary Distribution Plan

> Goal: Make `mcp-agent-mail` installable via `cargo install` as a single self-contained binary with embedded web UI.

## Current State Analysis

### Workspace Structure
```
mcp-agent-mail-rs/
├── crates/
│   ├── libs/lib-core/          # Domain logic (lib)
│   └── services/
│       ├── mcp-server/         # REST API binary (Axum)
│       ├── mcp-stdio/          # MCP protocol binary (rmcp)
│       ├── mcp-cli/            # CLI binary
│       └── web-ui/             # SvelteKit (NOT Rust)
├── migrations/                 # Already embedded via include_str!()
└── data/                       # Runtime-generated (DB + Git archive)
```

### Problems for `cargo install`

| Problem | Current State | Impact |
|---------|--------------|--------|
| **Web UI not embedded** | SvelteKit outputs to `web-ui/build/` | Binary can't serve frontend |
| **No static file serving** | `mcp-server` only has JSON API routes | No `/mail/*` routes |
| **Hardcoded data path** | `data/mcp_agent_mail.db` | Fails in non-project directories |
| **Workspace structure** | 4 separate crates | Can't install single binary from crates.io |
| **Build dependency** | Requires `bun` to build frontend | `cargo install` can't run npm/bun |

---

## Proposed Architecture

### Target: Unified Binary

```
mcp-agent-mail (single binary)
├── serve      # HTTP server (REST API + Web UI)
├── mcp        # MCP protocol server (stdio/SSE)
├── cli        # CLI commands (inbox, send, etc.)
└── init       # Initialize data directory
```

### Package Structure

```
mcp-agent-mail-rs/
├── Cargo.toml              # Single package (not workspace)
├── src/
│   ├── main.rs             # Unified CLI entry point
│   ├── lib.rs              # Core library (from lib-core)
│   ├── server/             # Axum server module
│   │   ├── mod.rs
│   │   ├── api.rs          # REST endpoints
│   │   └── static_files.rs # Embedded web UI serving
│   ├── mcp/                # MCP protocol module
│   │   └── mod.rs
│   ├── cli/                # CLI commands module
│   │   └── mod.rs
│   ├── model/              # Domain models (from lib-core)
│   ├── store/              # Storage layer (from lib-core)
│   └── embedded/           # Embedded assets
│       └── web_ui.rs       # Generated: embedded web UI files
├── migrations/
│   └── 001_initial_schema.sql
├── build.rs                # Build script to embed web UI
└── web-ui/                 # SvelteKit source (for development)
```

---

## Implementation Plan

### Phase 1: Restructure to Single Crate

**Tasks:**

#### 1.1 Flatten Workspace to Single Crate
```bash
# Current: 4 crates in workspace
# Target: 1 crate with modules
```

- [ ] Create new `src/lib.rs` combining `lib-core` exports
- [ ] Move `lib-core/src/model/*` → `src/model/`
- [ ] Move `lib-core/src/store/*` → `src/store/`
- [ ] Move `lib-core/src/ctx/*` → `src/ctx/`
- [ ] Move `mcp-server/src/*` → `src/server/`
- [ ] Move `mcp-stdio/src/*` → `src/mcp/`
- [ ] Update all `use` statements

#### 1.2 Create Unified Main Entry Point
```rust
// src/main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mcp-agent-mail")]
#[command(about = "Gmail for coding agents - MCP-compliant mail server")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start HTTP server (REST API + Web UI)
    Serve {
        #[arg(short, long, default_value = "8000")]
        port: u16,
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
    /// Start MCP protocol server
    Mcp {
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    /// Initialize data directory
    Init {
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// CLI commands for direct interaction
    #[command(subcommand)]
    Cli(CliCommands),
}
```

#### 1.3 Update Cargo.toml
```toml
[package]
name = "mcp-agent-mail"
version = "0.1.0"
edition = "2021"
description = "Gmail for coding agents - MCP-compliant mail server"
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-org/mcp-agent-mail-rs"
keywords = ["mcp", "ai", "agents", "mail", "coordination"]
categories = ["command-line-utilities", "web-programming"]
readme = "README.md"

[[bin]]
name = "mcp-agent-mail"
path = "src/main.rs"

[lib]
name = "mcp_agent_mail"
path = "src/lib.rs"

[features]
default = ["server", "mcp", "cli", "web-ui"]
server = []           # HTTP server support
mcp = ["rmcp"]        # MCP protocol support
cli = []              # CLI commands
web-ui = []           # Embedded web UI (adds ~300KB)

[dependencies]
# ... all dependencies from workspace
```

---

### Phase 2: Embed Web UI Assets

**Strategy**: Use `rust-embed` or `include_dir` to embed pre-built SvelteKit assets.

#### 2.1 Add Build Script
```rust
// build.rs
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=web-ui/src");
    println!("cargo:rerun-if-changed=web-ui/package.json");

    let out_dir = env::var("OUT_DIR").unwrap();
    let web_ui_dir = Path::new("web-ui");
    let build_dir = web_ui_dir.join("build");

    // Only rebuild if source changed or build doesn't exist
    if !build_dir.exists() || cfg!(feature = "rebuild-web-ui") {
        // Check if bun/npm is available
        let has_bun = Command::new("bun").arg("--version").output().is_ok();
        let has_npm = Command::new("npm").arg("--version").output().is_ok();

        if has_bun {
            Command::new("bun").args(["install"]).current_dir(web_ui_dir).status().ok();
            Command::new("bun").args(["run", "build"]).current_dir(web_ui_dir).status().ok();
        } else if has_npm {
            Command::new("npm").args(["install"]).current_dir(web_ui_dir).status().ok();
            Command::new("npm").args(["run", "build"]).current_dir(web_ui_dir).status().ok();
        } else {
            println!("cargo:warning=No bun or npm found. Using pre-built web-ui if available.");
        }
    }

    // Generate embedded assets module
    generate_embedded_assets(&build_dir, &out_dir);
}

fn generate_embedded_assets(build_dir: &Path, out_dir: &str) {
    // ... generate Rust code to embed files
}
```

#### 2.2 Use rust-embed Crate
```toml
[dependencies]
rust-embed = { version = "8", features = ["interpolate-folder-path"] }
mime_guess = "2"
```

```rust
// src/embedded/web_ui.rs
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web-ui/build/"]
#[prefix = ""]
pub struct WebUiAssets;
```

#### 2.3 Implement Static File Serving
```rust
// src/server/static_files.rs
use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
    response::IntoResponse,
};
use crate::embedded::WebUiAssets;

pub async fn serve_static(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');

    // Try exact path first
    if let Some(content) = WebUiAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(content.data.into_owned()))
            .unwrap();
    }

    // SvelteKit SPA fallback: serve index.html for unmatched routes
    if let Some(content) = WebUiAssets::get("index.html") {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data.into_owned()))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap()
}
```

#### 2.4 Integrate with Router
```rust
// src/server/mod.rs
use axum::{routing::get, Router};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // API routes
        .nest("/api", api_routes())
        // Health/metrics
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        // Static files (web UI) - must be last (fallback)
        .fallback(serve_static)
        .with_state(state)
}
```

---

### Phase 3: Configurable Data Directory

**Problem**: Current code hardcodes `data/mcp_agent_mail.db`

#### 3.1 Add Configuration System
```rust
// src/config.rs
use std::path::PathBuf;
use directories::ProjectDirs;

#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub archive_path: PathBuf,
}

impl Config {
    pub fn new(custom_path: Option<PathBuf>) -> Self {
        let data_dir = custom_path.unwrap_or_else(|| {
            // Use XDG/platform-specific directories
            if let Some(proj_dirs) = ProjectDirs::from("io", "mcp", "agent-mail") {
                proj_dirs.data_dir().to_path_buf()
            } else {
                // Fallback to current directory
                PathBuf::from("data")
            }
        });

        Config {
            db_path: data_dir.join("mcp_agent_mail.db"),
            archive_path: data_dir.join("archive"),
            data_dir,
        }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.archive_path)?;
        Ok(())
    }
}
```

#### 3.2 Add directories Dependency
```toml
[dependencies]
directories = "5"
```

#### 3.3 Update Store to Use Config
```rust
// src/store/mod.rs
pub async fn new_db_pool(config: &Config) -> Result<Db> {
    config.ensure_dirs()?;

    let db = Builder::new_local(&config.db_path).build().await?;
    let conn = db.connect()?;

    conn.execute("PRAGMA journal_mode=WAL;", ()).await?;

    let schema = include_str!("../../migrations/001_initial_schema.sql");
    conn.execute_batch(schema).await?;

    Ok(conn)
}
```

---

### Phase 4: Pre-built Web UI for crates.io

**Problem**: `cargo install` cannot run `bun build` during installation.

#### Solution: Include Pre-built Assets in Repository

```bash
# In CI/release workflow:
cd web-ui && bun install && bun run build
git add web-ui/build/
git commit -m "chore: update pre-built web UI assets"
```

#### 4.1 Add to .gitignore Exceptions
```gitignore
# Ignore node_modules but keep build output
web-ui/node_modules/
!web-ui/build/
```

#### 4.2 Conditional Build Script
```rust
// build.rs
fn main() {
    // For crates.io: use pre-built assets
    // For development: rebuild if needed

    let build_dir = Path::new("web-ui/build");

    if build_dir.exists() && build_dir.join("index.html").exists() {
        // Pre-built assets exist, use them
        println!("cargo:warning=Using pre-built web UI assets");
        return;
    }

    // Development: try to build
    #[cfg(feature = "dev")]
    build_web_ui();
}
```

---

### Phase 5: Feature Flags for Optional Components

```toml
[features]
default = ["server", "mcp", "cli", "web-ui"]

# Core features
server = ["axum", "tower-http"]
mcp = ["rmcp"]
cli = ["clap"]

# Optional: Embedded web UI (adds ~300KB to binary)
web-ui = ["rust-embed", "mime_guess"]

# Development: rebuild web UI from source
dev = []
```

```rust
// src/main.rs
#[cfg(feature = "server")]
mod server;

#[cfg(feature = "mcp")]
mod mcp;

#[cfg(feature = "cli")]
mod cli;
```

---

### Phase 6: Release & Publishing

#### 6.1 GitHub Actions Workflow
```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build-web-ui:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - run: cd web-ui && bun install && bun run build
      - uses: actions/upload-artifact@v4
        with:
          name: web-ui-build
          path: web-ui/build/

  build-binaries:
    needs: build-web-ui
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          name: web-ui-build
          path: web-ui/build/
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: mcp-agent-mail-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/mcp-agent-mail*

  publish-crates-io:
    needs: build-binaries
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          name: web-ui-build
          path: web-ui/build/
      - run: cargo publish --token ${{ secrets.CARGO_TOKEN }}
```

#### 6.2 Installation Methods

After implementation, users can install via:

```bash
# From crates.io
cargo install mcp-agent-mail

# From git (latest)
cargo install --git https://github.com/your-org/mcp-agent-mail-rs

# Specific version
cargo install mcp-agent-mail@0.1.0

# Without web UI (smaller binary)
cargo install mcp-agent-mail --no-default-features --features server,mcp,cli

# From pre-built releases
curl -fsSL https://github.com/your-org/mcp-agent-mail-rs/releases/latest/download/mcp-agent-mail-$(uname -s)-$(uname -m) -o mcp-agent-mail
chmod +x mcp-agent-mail
```

---

## Task Checklist

### Phase 1: Restructure (Priority: P0)
- [ ] Create `src/lib.rs` with public API
- [ ] Move `model/` module (6 files)
- [ ] Move `store/` module (2 files)
- [ ] Move `ctx/` module (1 file)
- [ ] Move `server/` module from mcp-server
- [ ] Move `mcp/` module from mcp-stdio
- [ ] Create unified `main.rs` with clap subcommands
- [ ] Update all import paths
- [ ] Convert workspace Cargo.toml to single package
- [ ] Verify tests pass

### Phase 2: Embed Web UI (Priority: P0)
- [ ] Add `rust-embed` dependency
- [ ] Create `src/embedded/web_ui.rs`
- [ ] Implement `serve_static` handler
- [ ] Add fallback route to router
- [ ] Create `build.rs` for development builds
- [ ] Test embedded file serving

### Phase 3: Configurable Paths (Priority: P1)
- [ ] Add `directories` crate
- [ ] Create `src/config.rs`
- [ ] Update `store/mod.rs` to accept config
- [ ] Update `ModelManager` constructor
- [ ] Add `--data-dir` CLI flag
- [ ] Test XDG directory creation

### Phase 4: Pre-built Assets (Priority: P1)
- [ ] Build web UI: `cd web-ui && bun run build`
- [ ] Commit `web-ui/build/` to repository
- [ ] Update `.gitignore` to allow build/
- [ ] Verify `cargo build` works without bun

### Phase 5: Feature Flags (Priority: P2)
- [ ] Define feature flags in Cargo.toml
- [ ] Add `#[cfg(feature = "...")]` guards
- [ ] Test minimal build: `--no-default-features`
- [ ] Document feature combinations

### Phase 6: Release Pipeline (Priority: P2)
- [ ] Create GitHub Actions workflow
- [ ] Build multi-platform binaries
- [ ] Publish to crates.io
- [ ] Create release with binaries
- [ ] Update README with install instructions

---

## Binary Size Estimates

| Configuration | Estimated Size |
|--------------|----------------|
| Full (server + mcp + cli + web-ui) | ~15-20 MB |
| Without web-ui | ~12-15 MB |
| Minimal (server only) | ~8-10 MB |
| After `strip` and UPX | ~5-8 MB |

---

## Migration Path

### For Existing Users

```bash
# Old way (workspace)
cargo run -p mcp-server
cargo run -p mcp-stdio -- serve

# New way (single binary)
mcp-agent-mail serve        # HTTP server
mcp-agent-mail mcp          # MCP protocol
mcp-agent-mail cli inbox    # CLI commands
```

### Backward Compatibility

- Keep old binary names as aliases (via symlinks or shell scripts)
- Environment variables remain unchanged
- API endpoints unchanged
- Data directory auto-migrates

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Large binary size from embedded assets | Download time, disk space | Feature flag to exclude web-ui |
| Build complexity with build.rs | CI failures | Pre-commit web UI builds |
| Breaking changes to API | Existing integrations | Semantic versioning, changelog |
| Cross-compilation issues with git2 | Limited platform support | Document supported platforms |

---

## Success Criteria

- [ ] `cargo install mcp-agent-mail` works on Linux/macOS/Windows
- [ ] Single binary serves REST API + Web UI + MCP protocol
- [ ] Binary size under 25MB
- [ ] No runtime dependencies on npm/bun/node
- [ ] Data stored in XDG-compliant directory
- [ ] All existing tests pass
