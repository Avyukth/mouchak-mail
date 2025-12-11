# Critical Production Hardening Plan

**Date**: 2025-12-11
**Project**: mcp-agent-mail-rs
**Status**: Pre-Production (Phase 1.5 complete)

---

## Executive Summary

### Current State

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| **PMAT Score** | 83.5/134 (62.3%) | ≥100/134 (75%) | ⚠️ B+ |
| **Quality Gate** | 26 violations | 0 violations | ❌ FAILED |
| **Test Coverage** | 12.5% | ≥85% | ❌ Critical |
| **Documentation** | 33.3% | ≥80% | ❌ Poor |
| **Security Audit** | 1 warning | 0 warnings | ⚠️ |
| **unwrap() Calls** | 20+ (tests) | 0 (prod code) | ⚠️ |

### Critical Blockers for Production

1. **No unified CLI** - Cannot run `serve-http` like Python version
2. **No installer script** - No easy deployment path
3. **Quality gate failing** - 26 violations must be resolved
4. **Test coverage 12.5%** - Unacceptable for production
5. **Dead code warnings** - 5 unused struct fields

---

## PMAT Analysis Summary

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🦀  Rust Project Score: 83.5/134 (62.3%) Grade B+
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Category Breakdown:
├── Code Quality:        20.0/26 (76.9%) ⚠️
├── Dependency Health:    6.0/12 (50.0%) ❌
├── Documentation:        5.0/15 (33.3%) ❌
├── Formal Verification:  3.0/8  (37.5%) ❌
├── Known Defects:       20.0/20 (100%) ✅
├── Performance:          0.0/10 (0.0%)  ❌
├── Rust Tooling/CI:     27.0/130 (20.8%) ❌
└── Testing Excellence:   2.5/20 (12.5%) ❌
```

### Quality Gate Violations (26 Total)

| Category | Count | Severity |
|----------|-------|----------|
| Complexity | 7 | Medium |
| Dead Code | 5 | Low |
| Technical Debt (SATD) | 3 | Low |
| Code Entropy | 5 | Low |
| Duplicates | 2 | Low |
| Documentation | 3 | Medium |
| Provability | 1 | Low |

### Complexity Hotspots

| Function | File | Cyclomatic | Action |
|----------|------|------------|--------|
| `commit_file` | git_store.rs:100 | 14 | Refactor |
| `create_agent_identity` | tools.rs:550 | 13 | Refactor |
| `main` | mcp-cli/main.rs | 12 | Refactor |
| `commit_paths` | git_store.rs:150 | 11 | Refactor |
| `file_reservation_paths` | tools.rs:500 | 11 | Review |

---

## Priority Matrix

### P0 - Critical (Blocking Production)

| Issue | Impact | Effort | Owner |
|-------|--------|--------|-------|
| Create unified CLI binary | Cannot deploy as replacement | 2d | - |
| Create installer script | No easy deployment | 1d | - |
| Fix dead code warnings | Build warnings | 2h | - |
| Document beads env vars | Users can't configure | 1h | - |
| Add route aliases | Python API incompatibility | 4h | - |

### P1 - High (Required for Production)

| Issue | Impact | Effort | Owner |
|-------|--------|--------|-------|
| Increase test coverage to 85% | Risk of regressions | 5d | - |
| Refactor complexity hotspots | Maintainability | 2d | - |
| Add integration tests | End-to-end validation | 3d | - |
| Implement pre-commit guard | File reservation enforcement | 1d | - |
| Add structured logging | Observability | 1d | - |
| Add metrics endpoint | Monitoring | 4h | - |

### P2 - Medium (Recommended)

| Issue | Impact | Effort | Owner |
|-------|--------|--------|-------|
| Add bearer token auth | Security for remote access | 1d | - |
| Complete attachment handlers | Feature completeness | 2d | - |
| Add graceful shutdown | Clean termination | 4h | - |
| Add connection pooling config | Performance | 4h | - |
| Add rate limiting | DoS protection | 4h | - |

### P3 - Low (Nice to Have)

| Issue | Impact | Effort | Owner |
|-------|--------|--------|-------|
| Thread summarization (LLM) | Advanced feature | 3d | - |
| Project sibling suggestions | AI-assisted discovery | 2d | - |
| Performance benchmarks | Optimization baseline | 1d | - |
| Documentation coverage 80% | Developer experience | 2d | - |

---

## Detailed Action Plan

### Phase 1: Critical Fixes (Week 1)

#### 1.1 Unified CLI Binary

Create `crates/bins/mcp-agent-mail/` with subcommands:

```rust
// src/main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mcp-agent-mail")]
#[command(about = "MCP Agent Mail Server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start HTTP server
    ServeHttp {
        #[arg(short, long, default_value = "8765")]
        port: u16,
    },
    /// Start MCP stdio server
    ServeMcp,
    /// Health check
    Health {
        #[arg(short, long, default_value = "http://127.0.0.1:8765")]
        url: String,
    },
    /// Show version
    Version,
}
```

#### 1.2 Installer Script

Create `scripts/install.sh`:

```bash
#!/bin/bash
set -e

VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS/arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case $ARCH in
    x86_64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
esac

# Download binary
BINARY_URL="https://github.com/.../releases/download/${VERSION}/mcp-agent-mail-${OS}-${ARCH}"
curl -fsSL "$BINARY_URL" -o "$INSTALL_DIR/mcp-agent-mail"
chmod +x "$INSTALL_DIR/mcp-agent-mail"

# Create launchd/systemd service
if [[ "$OS" == "darwin" ]]; then
    # macOS launchd plist
    cat > ~/Library/LaunchAgents/com.mcp-agent-mail.plist << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "...">
<plist version="1.0">
<dict>
    <key>Label</key><string>com.mcp-agent-mail</string>
    <key>ProgramArguments</key>
    <array>
        <string>$INSTALL_DIR/mcp-agent-mail</string>
        <string>serve-http</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
</dict>
</plist>
EOF
    launchctl load ~/Library/LaunchAgents/com.mcp-agent-mail.plist
fi

# Verify
curl -s http://127.0.0.1:8765/health
echo "✅ MCP Agent Mail installed and running"
```

#### 1.3 Fix Dead Code Warnings

```rust
// Remove unused fields or add #[allow(dead_code)] with justification

// tools.rs - Remove GetMessagePayload if unused
// Or use it in the appropriate route handler

// tools.rs:1524 - Remove params field or implement macro invocation
// tools.rs:1758 - Remove mime_type or implement attachment handling

// mcp-stdio/tools.rs:551 - Implement hint usage in create_agent_identity
// mcp-stdio/tools.rs:751 - Implement include_attachments in export
```

#### 1.4 Environment Variable Support

```rust
// In mcp-server/src/main.rs
let port = std::env::var("PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(8765);

let host = std::env::var("HOST")
    .unwrap_or_else(|_| "127.0.0.1".to_string());

// Document in README:
// BEADS_AGENT_MAIL_URL - Server URL (default: http://127.0.0.1:8765)
// BEADS_AGENT_NAME - Agent identifier
// BEADS_PROJECT_ID - Project namespace
// PORT - Server port (default: 8765)
// HOST - Bind address (default: 127.0.0.1)
```

#### 1.5 Route Aliases for Python Compatibility

```rust
// In api.rs - Add GET aliases for Python compatibility
.route("/api/reservations", get(list_reservations_compat))
.route("/api/reservations/:id", delete(release_reservation_compat))
.route("/api/inbox/:agent", get(inbox_by_agent_compat))
.route("/api/thread/:id", get(thread_by_id_compat))

// Implement thin wrappers that call existing POST handlers
async fn list_reservations_compat(
    State(mm): State<ModelManager>,
) -> impl IntoResponse {
    // Delegate to existing list_file_reservations
}
```

---

### Phase 2: Quality & Testing (Week 2-3)

#### 2.1 Test Coverage Target: 85%

```bash
# Install coverage tool
cargo install cargo-llvm-cov

# Run coverage
cargo llvm-cov --workspace --html

# Current: 12.5% → Target: 85%
```

**Test Strategy:**

| Layer | Current | Target | Focus |
|-------|---------|--------|-------|
| lib-core/model | ~20% | 90% | BMC CRUD operations |
| lib-core/store | ~10% | 85% | Git + SQLite |
| mcp-server/tools | ~5% | 80% | API handlers |
| mcp-stdio | ~15% | 80% | MCP protocol |

**Priority Test Cases:**

1. **Message lifecycle**: create → send → inbox → read → ack
2. **File reservations**: reserve → check → renew → release
3. **Agent management**: register → whois → profile → contacts
4. **Search**: FTS5 queries, ranking, pagination
5. **Git archive**: commit, message storage, agent profiles

#### 2.2 Refactor Complexity Hotspots

**commit_file (cyclomatic: 14)**
```rust
// BEFORE: Single monolithic function
pub fn commit_file(&self, path: &Path, message: &str) -> Result<Oid> {
    // 14 branches, 150+ lines
}

// AFTER: Extract helper functions
pub fn commit_file(&self, path: &Path, message: &str) -> Result<Oid> {
    let tree = self.build_tree_for_path(path)?;
    let parent = self.get_head_commit()?;
    self.create_commit(&tree, &parent, message)
}

fn build_tree_for_path(&self, path: &Path) -> Result<Tree> { ... }
fn get_head_commit(&self) -> Result<Option<Commit>> { ... }
fn create_commit(&self, tree: &Tree, parent: &Option<Commit>, msg: &str) -> Result<Oid> { ... }
```

**create_agent_identity (cyclomatic: 13)**
```rust
// Extract name generation logic
fn generate_memorable_name(existing: &[String], hint: Option<&str>) -> String {
    // Move ADJECTIVES × NOUNS logic here
}

// Extract collision detection
fn find_unique_name(mm: &ModelManager, project_id: i64, base: &str) -> Result<String> {
    // Move uniqueness check here
}
```

#### 2.3 Integration Tests

Create `tests/integration/` with end-to-end scenarios:

```rust
// tests/integration/messaging_flow.rs
#[tokio::test]
async fn test_complete_messaging_flow() {
    let app = spawn_test_server().await;

    // 1. Create project
    let project = app.ensure_project("test-project").await;

    // 2. Register agents
    let alice = app.register_agent(&project, "alice").await;
    let bob = app.register_agent(&project, "bob").await;

    // 3. Send message
    let msg = app.send_message(&alice, &[&bob], "Hello").await;

    // 4. Check inbox
    let inbox = app.check_inbox(&bob).await;
    assert_eq!(inbox.len(), 1);

    // 5. Acknowledge
    app.acknowledge_message(&bob, &msg.id).await;

    // 6. Search
    let results = app.search_messages(&project, "Hello").await;
    assert_eq!(results.len(), 1);
}
```

---

### Phase 3: Production Hardening (Week 4)

#### 3.1 Security Hardening

```rust
// Add bearer token authentication
async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = std::env::var("HTTP_BEARER_TOKEN").ok();
    let localhost_unauth = std::env::var("HTTP_ALLOW_LOCALHOST_UNAUTHENTICATED")
        .map(|v| v == "true")
        .unwrap_or(false);

    // Allow localhost without auth in dev mode
    if localhost_unauth && is_localhost(&request) {
        return Ok(next.run(request).await);
    }

    // Require bearer token
    match (token, headers.get("Authorization")) {
        (Some(expected), Some(provided)) => {
            if provided.to_str().ok() == Some(&format!("Bearer {}", expected)) {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        (None, _) => Ok(next.run(request).await), // No token configured
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
```

#### 3.2 Observability

```rust
// Structured logging with tracing
use tracing::{info, error, warn, instrument};

#[instrument(skip(mm), fields(project = %payload.project_slug))]
pub async fn ensure_project(
    State(mm): State<ModelManager>,
    Json(payload): Json<EnsureProjectPayload>,
) -> Result<Json<Project>, AppError> {
    info!("Ensuring project exists");
    let project = ProjectBmc::ensure(&ctx, &mm, &payload.project_slug).await?;
    info!(project_id = %project.id, "Project ensured");
    Ok(Json(project))
}

// Metrics with prometheus
use metrics::{counter, histogram};

pub async fn send_message(...) -> Result<...> {
    let start = Instant::now();
    counter!("messages_sent_total").increment(1);

    let result = MessageBmc::create(...).await;

    histogram!("message_send_duration_seconds").record(start.elapsed().as_secs_f64());
    result
}
```

#### 3.3 Graceful Shutdown

```rust
// In main.rs
let listener = TcpListener::bind(&addr).await?;
info!("Server listening on {}", addr);

axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown");
}
```

#### 3.4 Rate Limiting

```rust
use tower_governor::{GovernorLayer, GovernorConfigBuilder};

let governor_conf = GovernorConfigBuilder::default()
    .per_second(10)
    .burst_size(50)
    .finish()
    .unwrap();

let app = Router::new()
    .route("/api/messages/send", post(send_message))
    .layer(GovernorLayer { config: governor_conf });
```

---

### Phase 4: Documentation & CI/CD (Week 5)

#### 4.1 API Documentation

```rust
// Add OpenAPI docs with utoipa
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        ensure_project,
        register_agent,
        send_message,
        check_inbox,
        // ... all endpoints
    ),
    components(schemas(
        Project,
        Agent,
        Message,
        FileReservation,
    ))
)]
struct ApiDoc;

// Serve Swagger UI
.route("/docs", get(|| async { Html(swagger_ui()) }))
.route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
```

#### 4.2 CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --workspace -- -D warnings

      - name: Audit
        run: cargo audit

      - name: PMAT Quality Gate
        run: |
          cargo install pmat
          pmat quality-gate --strict
          pmat rust-project-score --min 100

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Run tests
        run: cargo test --workspace

      - name: Coverage
        run: |
          cargo install cargo-llvm-cov
          cargo llvm-cov --workspace --lcov --output-path lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
          fail_ci_if_error: true
          threshold: 85%

  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
    steps:
      - uses: actions/checkout@v4

      - name: Build release
        run: cargo build --release --target ${{ matrix.target }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: mcp-agent-mail-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/mcp-agent-mail
```

---

## Quality Gates Checklist

### Before Merge

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo audit` shows no vulnerabilities
- [ ] `pmat quality-gate` passes
- [ ] Test coverage ≥85%
- [ ] No `unwrap()` in production code paths
- [ ] All new public APIs documented

### Before Release

- [ ] PMAT score ≥100/134
- [ ] All P0 and P1 issues resolved
- [ ] Integration tests pass
- [ ] Load testing completed
- [ ] Security review completed
- [ ] README updated
- [ ] CHANGELOG updated
- [ ] Version bumped

---

## Monitoring Checklist (Post-Deploy)

- [ ] Health endpoint responding: `GET /health`
- [ ] Metrics endpoint active: `GET /metrics`
- [ ] Logs flowing to aggregator
- [ ] Alerts configured for:
  - [ ] Error rate > 1%
  - [ ] Latency p99 > 500ms
  - [ ] Memory usage > 80%
  - [ ] Disk usage > 90%

---

## Timeline Summary

| Week | Focus | Deliverables |
|------|-------|--------------|
| 1 | Critical Fixes | CLI, installer, dead code, env vars |
| 2-3 | Quality & Testing | 85% coverage, refactoring, integration tests |
| 4 | Production Hardening | Security, observability, graceful shutdown |
| 5 | Documentation & CI/CD | OpenAPI docs, CI pipeline, release automation |

**Total Estimated Effort**: 5 weeks (1 engineer)

---

## Success Criteria

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| PMAT Score | 83.5 | ≥100 | Pending |
| Quality Gate | FAILED | PASSED | Pending |
| Test Coverage | 12.5% | ≥85% | Pending |
| Documentation | 33.3% | ≥80% | Pending |
| Security Audit | 1 warning | 0 | Pending |
| unwrap() in prod | 0 | 0 | ✅ |
| CLI compatibility | ❌ | ✅ | Pending |
| Installer | ❌ | ✅ | Pending |

---

## References

- [Gap Analysis](./GAP_ANALYSIS.md) - Python vs Rust comparison
- [AGENTS.md](../AGENTS.md) - Project instructions
- [Production Hardening Skill](~/.claude/skills/production-hardening-backend)
- [Rust Skills](~/.claude/skills/rust-skills)
- [PMAT Documentation](https://github.com/paiml/pmat)
