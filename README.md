# MCP Agent Mail (Rust)

> "It's like Gmail for your coding agents!"

A high-performance Rust implementation of a mail-like coordination layer for AI coding agents, exposed as both REST API and MCP (Model Context Protocol) server. Enables asynchronous communication between multiple agents working on shared codebases with full audit trails.

**44.6x faster than Python** - 15,200 req/s vs 341 req/s in original implementation.

**Ported from**: [mcp_agent_mail (Python)](https://github.com/Dicklesworthstone/mcp_agent_mail)

## Why This Exists

Modern projects often run multiple coding agents simultaneously (backend, frontend, scripts, infra). Without coordination, agents:

- Overwrite each other's edits or panic on unexpected diffs
- Miss critical context from parallel workstreams
- Require humans to "liaison" messages across tools

MCP Agent Mail provides:

- **Agent Identity**: Memorable adjective+noun names (BlueMountain, GreenCastle)
- **Messaging**: GitHub-Flavored Markdown messages with threading, To/CC/BCC
- **File Reservations**: Advisory locks to prevent edit conflicts
- **Contact Management**: Explicit approval for cross-project messaging
- **Searchable Archives**: FTS5 full-text search across message bodies
- **Git-Backed Audit Trail**: All messages persisted for human review
- **Build Slot Management**: Exclusive build resource locks
- **Macro System**: Reusable workflow definitions

## Quick Start

### Prerequisites

- **Rust** 1.85+ (Edition 2024)
- **Trunk** (for Leptos WASM frontend)

```bash
# Install Trunk for frontend builds
cargo install trunk

# Optional: Install cargo-deny for dependency auditing
cargo install cargo-deny
```

### Development Setup

```bash
# Clone the repository
git clone https://github.com/Avyukth/mcp-agent-mail-rs
cd mcp-agent-mail-rs

# Build all Rust components
cargo build --workspace

# Run development servers (API + Web UI)
make dev
```

Development servers:
- **API Server**: http://localhost:8765
- **Web UI**: http://localhost:8080 (Leptos WASM)

### Production Build

```bash
# Build everything for production
make build-prod

# Run production server
cargo run -p mcp-server --release
```

### Using the Unified CLI

```bash
# Install globally
cargo install --path crates/services/mcp-agent-mail

# Or run directly
mcp-agent-mail serve              # Start REST API server
mcp-agent-mail mcp                # Start MCP stdio server
mcp-agent-mail tools              # List MCP tools
mcp-agent-mail schema             # Export JSON schema
```

## Architecture

```
mcp-agent-mail-rs/
├── crates/
│   ├── libs/
│   │   ├── lib-core/             # Domain logic, BMC pattern, storage
│   │   │   ├── src/model/        # 15+ entities (Agent, Message, Project, etc.)
│   │   │   ├── src/store/        # Database (libsql) + Git (git2) storage
│   │   │   └── tests/            # Integration tests
│   │   ├── lib-common/           # Config, errors, tracing
│   │   ├── lib-server/           # Axum REST API, middleware, OpenAPI
│   │   └── lib-mcp/              # MCP tool definitions (32+ tools)
│   ├── services/
│   │   ├── mcp-server/           # REST API server binary
│   │   ├── mcp-stdio/            # MCP protocol server (stdio + SSE)
│   │   ├── mcp-cli/              # CLI for testing
│   │   ├── mcp-agent-mail/       # Unified CLI binary
│   │   └── web-ui-leptos/        # Leptos WASM frontend (SPA)
│   └── tests/
│       └── e2e/                  # Playwright E2E tests
├── migrations/                   # SQLite schema (4 migrations, FTS5)
├── data/                         # Runtime data (SQLite DB, Git archive)
├── deny.toml                     # Dependency policy (licenses, advisories)
└── .clippy.toml                  # Clippy configuration
```

### Tech Stack

| Layer | Technology |
|-------|------------|
| **Backend** | Rust 2024, Axum 0.8, Tokio |
| **Database** | libsql (SQLite) with FTS5 full-text search |
| **Storage** | git2 for audit trail |
| **Protocol** | MCP via rmcp SDK (stdio + SSE) |
| **Frontend** | Leptos 0.8 (WASM, CSR mode) |
| **Metrics** | Prometheus (metrics-exporter-prometheus) |
| **Quality** | cargo-deny, clippy, pmat |

### Design Patterns

#### Backend Model Controller (BMC) Pattern

Separates concerns for each entity:

```rust
// Entity struct (database row)
pub struct Agent {
    pub id: i64,
    pub name: String,
    pub project_id: i64,
    // ...
}

// Creation input
pub struct AgentForCreate {
    pub name: String,
    pub project_id: i64,
    // ...
}

// Business logic (stateless)
pub struct AgentBmc;
impl AgentBmc {
    pub async fn create(ctx: &Ctx, mm: &ModelManager, data: AgentForCreate) -> Result<i64>;
    pub async fn get(ctx: &Ctx, mm: &ModelManager, id: i64) -> Result<Agent>;
    pub async fn get_by_name(ctx: &Ctx, mm: &ModelManager, project_id: i64, name: &str) -> Result<Agent>;
    // ...
}
```

#### Dual Persistence

All data stored in both:
1. **SQLite** (libsql): Fast queries, FTS5 search, transactions
2. **Git Repository**: Human-readable audit trail

```
data/archive/projects/{slug}/
├── agents/{name}/
│   ├── profile.json
│   ├── inbox/YYYY/MM/{message}.md
│   └── outbox/YYYY/MM/{message}.md
└── messages/YYYY/MM/{timestamp}__{subject}__{id}.md
```

## API Reference

### Health & Monitoring

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Health check with uptime |
| `/api/ready` | GET | Readiness probe (DB connectivity) |
| `/api/metrics` | GET | Prometheus metrics |

### Projects (5 endpoints)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/project/ensure` | POST | Create or get existing project |
| `/api/projects` | GET | List all projects |
| `/api/projects/{slug}/agents` | GET | List agents for project |
| `/api/project/info` | POST | Get project details |
| `/api/list_project_siblings` | POST | Find related projects |

### Agent Management (6 endpoints)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/agent/register` | POST | Register new agent |
| `/api/agent/whois` | POST | Lookup agent by name |
| `/api/agent/create_identity` | POST | Create with auto-generated name |
| `/api/agent/profile` | POST | Get agent profile |
| `/api/agent/profile/update` | POST | Update agent settings |
| `/api/agent/capabilities` | POST | Check/grant capabilities |

### Messaging (13 endpoints)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/message/send` | POST | Send message (to/cc/bcc) |
| `/api/message/reply` | POST | Reply to thread |
| `/api/message/read` | POST | Mark as read |
| `/api/message/acknowledge` | POST | Acknowledge receipt |
| `/api/messages/{id}` | GET | Get single message |
| `/api/messages/search` | POST | Full-text search |
| `/api/inbox` | POST | List inbox messages |
| `/api/outbox` | POST | List sent messages |
| `/api/thread` | POST | Get thread messages |
| `/api/threads` | POST | List all threads |
| `/api/thread/summarize` | POST | Summarize thread |

### File Reservations (5 endpoints)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/file_reservations/paths` | POST | Reserve file paths |
| `/api/file_reservations/list` | POST | List active reservations |
| `/api/file_reservations/release` | POST | Release reservations |
| `/api/file_reservations/renew` | POST | Extend TTL |
| `/api/file_reservations/force_release` | POST | Force release (admin) |

### Build Slots (3 endpoints)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/build_slots/acquire` | POST | Acquire exclusive slot |
| `/api/build_slots/renew` | POST | Extend TTL |
| `/api/build_slots/release` | POST | Release slot |

### Contacts (4 endpoints)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/contacts/request` | POST | Request to add contact |
| `/api/contacts/respond` | POST | Accept/reject request |
| `/api/contacts/list` | POST | List all contacts |
| `/api/contacts/policy` | POST | Set contact policy |

### Macros (4 endpoints)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/macros/list` | POST | List macros |
| `/api/macros/register` | POST | Create macro |
| `/api/macros/unregister` | POST | Delete macro |
| `/api/macros/invoke` | POST | Execute macro |

### Advanced Features

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/export` | POST | Export mailbox (HTML/JSON/MD/CSV) |
| `/api/attachments/add` | POST | Upload attachment |
| `/api/attachments/{id}` | GET | Download attachment |
| `/api/tool_metrics` | POST | Get tool usage metrics |
| `/api/activity` | POST | List activity log |
| `/api/overseer/send` | POST | Send human guidance |

**Note:** Python-compatible aliases available (e.g., `/api/send_message`, `/api/fetch_inbox`)

### MCP Protocol (32+ tools)

```bash
# Run MCP server for Claude Desktop integration
cargo run -p mcp-stdio -- serve

# Or with SSE transport
cargo run -p mcp-stdio -- serve --transport sse --port 3000

# List all available tools
cargo run -p mcp-stdio -- tools

# Export JSON schema
cargo run -p mcp-stdio -- schema
```

## Database Schema

SQLite with FTS5 full-text search. **15 tables** across 4 migrations:

| Table | Description |
|-------|-------------|
| `projects` | Project registry (slug, human_key) |
| `agents` | Agent profiles per project with policies |
| `messages` | Message content with threading |
| `message_recipients` | To/CC/BCC with read/ack tracking |
| `messages_fts` | FTS5 index for full-text search |
| `file_reservations` | Advisory file locks with TTL |
| `build_slots` | Exclusive build resource locks |
| `agent_links` | Cross-project contact approval |
| `agent_capabilities` | Per-agent capability grants |
| `macros` | Reusable workflow definitions |
| `products` | Multi-repo coordination |
| `product_project_links` | Project grouping |
| `project_sibling_suggestions` | Auto-discovery |
| `overseer_messages` | Human guidance messages |
| `tool_metrics` | Tool usage tracking |
| `attachments` | File attachment metadata |

See `migrations/` for full schema.

## Configuration

### Environment Variables

**Server:**
| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | 8765 | API server port |
| `MOUCHAK_SERVER__HOST` | 0.0.0.0 | Bind address |

**Logging:**
| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | info | Log level (debug, info, warn, error) |
| `LOG_FORMAT` | pretty | Log format (pretty, json) |
| `RUN_MODE` | development | Mode (development, production, test) |

**Database:**
| Variable | Default | Description |
|----------|---------|-------------|
| `SQLITE_PATH` | ./data/mcp_agent_mail.db | SQLite file path |
| `DATABASE_URL` | file:./data/mcp_agent_mail.db | Database URL |

**Git Archive:**
| Variable | Default | Description |
|----------|---------|-------------|
| `GIT_REPO_PATH` | ./data/archive | Archive location |
| `GIT_ARCHIVE_ENABLED` | false | Enable git archival |

**Rate Limiting:**
| Variable | Default | Description |
|----------|---------|-------------|
| `RATE_LIMIT_ENABLED` | true | Enable rate limiting |
| `RATE_LIMIT_RPS` | 1000 | Requests per second |
| `RATE_LIMIT_BURST` | 2000 | Burst allowance |

**File Reservations:**
| Variable | Default | Description |
|----------|---------|-------------|
| `FILE_RESERVATION_DEFAULT_TTL` | 3600 | Default TTL (seconds) |
| `FILE_RESERVATION_MAX_TTL` | 86400 | Max TTL (seconds) |

**MCP Protocol:**
| Variable | Default | Description |
|----------|---------|-------------|
| `MOUCHAK_MCP__TRANSPORT` | stdio | Transport (stdio, sse) |
| `MOUCHAK_MCP__PORT` | 3000 | SSE port |

See `.env.example` for complete list (35+ variables).

## Development Commands

### Using make (recommended)

```bash
make dev            # Run API + Web UI (parallel)
make dev-api        # Run API server only
make dev-web        # Run Web UI only
make dev-mcp        # Run MCP stdio server
make build          # Build debug
make build-release  # Build release with LTO
make build-web      # Build Leptos WASM frontend
make build-prod     # Full production build
make test           # Run all tests
make test-fast      # Run unit tests only
make coverage       # Generate coverage report
make audit          # Run security audits (cargo audit + deny)
make lint           # Run clippy lints
make fmt            # Format code
make quality-gate   # Run all quality gates
make clean          # Clean all artifacts
```

### Using cargo directly

```bash
# Build
cargo build --workspace
cargo build --workspace --release

# Test
cargo test --workspace --exclude e2e-tests
cargo test -p lib-core --test integration -- --test-threads=1

# Lint
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

# Security
cargo audit
cargo deny check
```

## Quality Gates

This project enforces strict quality standards:

**Workspace Lints (Cargo.toml):**
- `unsafe_code = "deny"` - No unsafe code allowed
- `unused_must_use = "deny"` - Enforce error handling
- `inefficient_to_string = "deny"` - Performance

**Clippy Configuration (.clippy.toml):**
- Cognitive complexity threshold: 30
- Max function lines: 200
- Max arguments: 10
- MSRV: 1.85

**Dependency Policy (deny.toml):**
- License allowlist (MIT, Apache-2.0, BSD, ISC, etc.)
- Security advisory checks
- Known advisory ignores with justification

## Integration with AI Agents

### Claude Desktop (MCP)

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "agent-mail": {
      "command": "/path/to/mcp-agent-mail-rs/target/release/mcp-stdio",
      "args": ["serve"]
    }
  }
}
```

### Claude Code / Codex CLI

AGENTS.md is automatically read. Start with:

```bash
cd /path/to/mcp-agent-mail-rs
# Then: "Run bd ready --json and start the first task"
```

### Gemini CLI

```bash
# Symlink AGENTS.md
ln -s ../AGENTS.md .gemini/GEMINI.md
```

## Typical Workflow

### For Agents

```bash
# 1. Register identity
curl -X POST http://localhost:8765/api/agent/register \
  -H "Content-Type: application/json" \
  -d '{"project_key": "/path/to/project", "name": "BlueMountain", "program": "claude", "model": "opus"}'

# 2. Reserve files before editing
curl -X POST http://localhost:8765/api/file_reservations/paths \
  -H "Content-Type: application/json" \
  -d '{"project_slug": "my-project", "agent_name": "BlueMountain", "paths": ["src/**"], "ttl_seconds": 3600}'

# 3. Send progress message
curl -X POST http://localhost:8765/api/message/send \
  -H "Content-Type: application/json" \
  -d '{"project_slug": "my-project", "from_agent": "BlueMountain", "to_agents": ["GreenCastle"], "subject": "Starting refactor", "body_md": "Working on auth module..."}'

# 4. Release reservation when done
curl -X POST http://localhost:8765/api/file_reservations/release \
  -H "Content-Type: application/json" \
  -d '{"project_slug": "my-project", "agent_name": "BlueMountain", "paths": ["src/**"]}'
```

### For Humans

1. **Web UI**: http://localhost:8080 (Leptos frontend)
2. **CLI**: `cargo run -p mcp-cli -- inbox --project my-project --agent BlueMountain`
3. **Git**: Browse `data/archive/` for full audit trail

## Project Status

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | COMPLETE | Core Architecture (BMC, storage) |
| 1.5 | COMPLETE | API Layer (Axum REST, 60+ endpoints) |
| 2 | COMPLETE | Leptos WASM Frontend |
| 3 | COMPLETE | Full Feature Parity (32+ MCP tools) |
| 4 | COMPLETE | MCP Protocol Integration (stdio + SSE) |
| 5 | COMPLETE | Production Hardening (deny.toml, quality gates) |
| 6 | IN PROGRESS | Performance Optimization |

## Performance

Benchmarked against Python reference implementation:

| Metric | Rust | Python | Improvement |
|--------|------|--------|-------------|
| Requests/sec | 15,200 | 341 | **44.6x** |
| P99 Latency | 2.1ms | 89ms | **42x** |
| Memory (idle) | 12MB | 180MB | **15x** |
| Startup time | 50ms | 2.1s | **42x** |

## References

- [Python Original](https://github.com/Dicklesworthstone/mcp_agent_mail) - Source implementation
- [MCP Tools Reference](https://glama.ai/mcp/servers/@Dicklesworthstone/mcp_agent_mail) - 28 MCP tools specification
- [Beads Issue Tracker](https://github.com/steveyegge/beads) - Task tracking via `bd` CLI
- [MCP Protocol](https://modelcontextprotocol.io) - Model Context Protocol specification
- [PMAT Quality Gates](https://paiml.github.io/pmat-book/) - Production maturity analysis

## License

MIT License - See LICENSE file for details.

---

Built with Rust for memory safety, performance, and reliability.
