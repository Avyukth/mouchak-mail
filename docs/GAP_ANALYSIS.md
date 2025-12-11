# Critical Gap Analysis: Rust vs Python MCP Agent Mail

**Date**: 2025-12-10
**Comparison**: `mcp-agent-mail-rs` vs `Dicklesworthstone/mcp_agent_mail`
**Reference Docs**: beads Agent Mail Quickstart, Multi-Workspace Setup, Deployment Guide

---

## Executive Summary

The Rust implementation is **~90% feature-complete** as a drop-in replacement for the Python version. Core messaging, file reservations, and multi-agent coordination are fully implemented. **Critical gaps** exist in installer tooling, CLI interface, and some advanced features.

| Category | Status | Priority |
|----------|--------|----------|
| Core Messaging | ✅ Complete | - |
| File Reservations | ✅ Complete | - |
| Agent Management | ✅ Complete | - |
| Full-Text Search | ✅ Complete | - |
| Git Archive | ✅ Complete | - |
| Web UI | ✅ Complete | - |
| MCP Protocol | ✅ Complete | - |
| **CLI Tool** | ⚠️ Missing | **P0** |
| **One-Line Installer** | ⚠️ Missing | **P0** |
| **Beads Integration** | ⚠️ Partial | **P1** |
| Pre-commit Guard | ⚠️ Stub | P2 |
| LLM Project Linking | ❌ Missing | P3 |

---

## Feature Comparison Matrix

### Core Features (Fully Implemented ✅)

| Feature | Python | Rust | Notes |
|---------|--------|------|-------|
| Project namespace management | ✅ | ✅ | `ensure_project`, `list_projects` |
| Agent registration | ✅ | ✅ | With memorable name generation |
| Message send/receive | ✅ | ✅ | Threading, importance levels |
| Message recipients (to/cc/bcc) | ✅ | ✅ | Full delivery tracking |
| Read/Acknowledge status | ✅ | ✅ | Per-recipient tracking |
| File reservations (advisory locks) | ✅ | ✅ | Exclusive + shared modes |
| Reservation TTL/expiry | ✅ | ✅ | Auto-expiration |
| Force release reservations | ✅ | ✅ | Emergency override |
| Full-text search (FTS5) | ✅ | ✅ | BM25 scoring |
| Git-backed persistence | ✅ | ✅ | Human-readable audit trail |
| SQLite indexing | ✅ | ✅ | libsql with FTS5 |
| Health/ready endpoints | ✅ | ✅ | `/health`, `/ready` |
| Metrics endpoint | ✅ | ✅ | Prometheus format |
| MCP stdio transport | ✅ | ✅ | Claude Desktop compatible |
| MCP SSE transport | ✅ | ✅ | Web client support |

### Extended Features (Implemented with Differences)

| Feature | Python | Rust | Gap Details |
|---------|--------|------|-------------|
| Thread summarization | ✅ (LLM) | ⚠️ (stub) | Endpoints exist, no LLM integration |
| Attachments | ✅ | ⚠️ (schema only) | Schema exists, handlers incomplete |
| Contact policies | ✅ | ✅ | `open/auto/contacts_only/block_all` |
| Cross-project contacts | ✅ | ✅ | Agent links with bidirectional status |
| Build slots | ❓ | ✅ | **Rust has MORE** - CI/CD isolation |
| Macros | ❓ | ✅ | **Rust has MORE** - Automation templates |
| Overseer messages | ✅ | ✅ | System notifications |
| Products (multi-repo) | ✅ | ✅ | Coordination units |
| Sibling suggestions | ✅ (LLM) | ⚠️ (schema) | Schema exists, no LLM linking |
| Export mailbox | ✅ | ⚠️ (partial) | Framework exists, formatters incomplete |

### Critical Missing Features (Gaps)

| Feature | Python | Rust | Impact | Priority |
|---------|--------|------|--------|----------|
| **CLI Tool** | ✅ `mcp_agent_mail.cli` | ❌ | Cannot run `serve-http` from command line | **P0** |
| **One-Line Installer** | ✅ `install.sh` | ❌ | No easy deployment path | **P0** |
| **Beads env vars** | ✅ Auto-detect | ⚠️ | `BEADS_AGENT_MAIL_URL` not documented | **P1** |
| Pre-commit guard | ✅ Full | ⚠️ Stub | Endpoints exist, no hook script | P2 |
| Web UI `/mail` | ✅ Full | ⚠️ Different | Rust has SvelteKit, not server-rendered | P2 |
| LLM project linking | ✅ Optional | ❌ | No AI-assisted sibling discovery | P3 |
| Human Overseer compose | ✅ | ⚠️ | Web UI compose exists, not identical | P3 |

---

## API Endpoint Comparison

### Matching Endpoints (Drop-in Compatible)

```
POST /api/project/ensure          ✅ Match
GET  /api/projects                ✅ Match
POST /api/agent/register          ✅ Match
POST /api/agent/whois             ✅ Match
POST /api/message/send            ✅ Match
POST /api/inbox                   ✅ Match
POST /api/messages/search         ✅ Match
POST /api/file_reservations/paths ✅ Match
POST /api/file_reservations/list  ✅ Match
POST /api/file_reservations/release ✅ Match
GET  /health                      ✅ Match
```

### Endpoint Naming Differences

| Python | Rust | Resolution |
|--------|------|------------|
| `GET /api/reservations` | `POST /api/file_reservations/list` | Add alias route |
| `DELETE /api/reservations/{id}` | `POST /api/file_reservations/release` | Add alias route |
| `GET /api/inbox/{agent}` | `POST /api/inbox` (body params) | Add GET alias |
| `GET /api/thread/{id}` | `POST /api/thread` (body params) | Add GET alias |

### Rust-Only Endpoints (Extended Features)

```
POST /api/build_slots/acquire     # CI/CD isolation (Rust extension)
POST /api/build_slots/release
POST /api/build_slots/renew
POST /api/macros/list             # Automation (Rust extension)
POST /api/macros/register
POST /api/macros/invoke
POST /api/contacts/policy         # Contact management (Rust extension)
```

---

## Environment Variable Compatibility

### Required for Beads Integration

| Variable | Python Support | Rust Support | Status |
|----------|----------------|--------------|--------|
| `BEADS_AGENT_MAIL_URL` | ✅ | ⚠️ Undocumented | Add to README |
| `BEADS_AGENT_NAME` | ✅ | ⚠️ Undocumented | Add to README |
| `BEADS_PROJECT_ID` | ✅ | ⚠️ Undocumented | Add to README |
| `HTTP_BEARER_TOKEN` | ✅ | ❓ Not implemented | Add auth middleware |
| `HTTP_ALLOW_LOCALHOST_UNAUTHENTICATED` | ✅ | ❓ Not implemented | Add dev mode |
| `PORT` | ✅ Default 8765 | ⚠️ CLI flag only | Add env var support |
| `LLM_ENABLED` | ✅ | ❌ | Not planned |

---

## Installation & Deployment Gap

### Python Installation (Current)
```bash
curl -fsSL "https://raw.githubusercontent.com/.../install.sh" | bash -s -- --yes
# Creates venv, installs deps, auto-starts server on port 8765
```

### Rust Installation (Missing)
```bash
# NEEDED: Similar one-liner
curl -fsSL "https://raw.githubusercontent.com/.../install.sh" | bash -s -- --yes
# Should: Download binary, create systemd/launchd service, start server
```

### Required Installer Features
1. **Binary distribution** - Pre-built binaries for macOS (arm64, x86), Linux
2. **Service installation** - launchd plist (macOS), systemd unit (Linux)
3. **Auto-start** - Start server immediately after install
4. **Health verification** - Confirm server responding on port 8765
5. **Claude MCP config** - Auto-update `~/.config/claude/mcp.json`

---

## CLI Interface Gap

### Python CLI
```bash
python -m mcp_agent_mail.cli serve-http --port 8765
python -m mcp_agent_mail.cli serve-mcp  # stdio mode
```

### Rust CLI (Needed)
```bash
mcp-agent-mail serve-http --port 8765
mcp-agent-mail serve-mcp
mcp-agent-mail --version
mcp-agent-mail health
```

Currently, Rust has:
- `mcp-server` binary (HTTP server) - **Works but different name**
- `mcp-stdio` binary (MCP server) - **Works but different name**
- `mcp-cli` - **Stub only**

**Resolution**: Create unified `mcp-agent-mail` binary with subcommands.

---

## Web UI Comparison

| Feature | Python (`/mail`) | Rust (SvelteKit) |
|---------|------------------|------------------|
| Server-rendered | ✅ Yes | ❌ SPA |
| Cross-project inbox | ✅ | ✅ |
| Per-project search | ✅ FTS5 | ✅ FTS5 |
| Agent directory | ✅ | ✅ |
| File reservation view | ✅ | ⚠️ Partial |
| Related projects | ✅ LLM-assisted | ❌ |
| Human Overseer compose | ✅ | ⚠️ Basic |
| PWA support | ❓ | ✅ |
| Mobile responsive | ❓ | ✅ |

**Verdict**: Different approach, both functional. Rust UI is more modern (PWA, Svelte 5).

---

## Database Schema Comparison

### Matching Tables
- `projects` ✅
- `agents` ✅
- `messages` ✅
- `message_recipients` ✅
- `file_reservations` ✅
- `messages_fts` (FTS5) ✅

### Rust Extensions (Not in Python)
- `build_slots` - CI/CD isolation slots
- `macros` - Automation templates
- `agent_links` - Enhanced contact management
- `products` - Multi-repo coordination
- `product_project_links` - Product-project associations
- `project_sibling_suggestions` - AI-assisted discovery (schema only)
- `overseer_messages` - System notifications

---

## MCP Tool Comparison

### Core Tools (Match)
| Tool | Python | Rust |
|------|--------|------|
| `register_agent` | ✅ | ✅ |
| `send_message` | ✅ | ✅ |
| `fetch_inbox` / `check_inbox` | ✅ | ✅ |
| `acknowledge_message` | ✅ | ✅ |
| `file_reservation_paths` | ✅ | ✅ |
| `release_file_reservations` | ✅ | ✅ |
| `search_messages` | ✅ | ✅ |

### Rust Extensions
- `acquire_build_slot`
- `release_build_slot`
- `register_macro`
- `invoke_macro`
- `request_contact`
- `respond_contact`
- `set_contact_policy`

---

## Critical Gaps: Action Items

### P0 - Must Fix for Drop-in Replacement

#### 1. Unified CLI Binary
```bash
# Create crates/bins/mcp-agent-mail/
cargo new --bin mcp-agent-mail
# Subcommands: serve-http, serve-mcp, health, version
```

#### 2. One-Line Installer
```bash
# Create scripts/install.sh
# - Detect OS/arch
# - Download binary from releases
# - Install to ~/.local/bin or /usr/local/bin
# - Create launchd/systemd service
# - Start server
# - Verify health
```

#### 3. Beads Compatibility Documentation
Update README with:
```markdown
## Beads Integration

Set these environment variables:
```bash
export BEADS_AGENT_MAIL_URL=http://127.0.0.1:8765
export BEADS_AGENT_NAME=my-agent
export BEADS_PROJECT_ID=my-project
```

The server is compatible with beads `bd` commands.
```

### P1 - Should Fix

#### 4. Add Route Aliases for Python Compatibility
```rust
// In api.rs
.route("/api/reservations", get(list_reservations_compat))
.route("/api/reservations/:id", delete(release_reservation_compat))
.route("/api/inbox/:agent", get(inbox_by_agent_compat))
.route("/api/thread/:id", get(thread_by_id_compat))
```

#### 5. Environment Variable Support
```rust
// In main.rs
let port = std::env::var("PORT")
    .unwrap_or("8765".to_string())
    .parse::<u16>()?;
```

### P2 - Nice to Have

#### 6. Pre-commit Guard Implementation
Create actual git hook script that checks reservations before commit.

#### 7. Bearer Token Authentication
Add optional middleware for production deployments.

---

## Rust Advantages (Beyond Python)

| Feature | Benefit |
|---------|---------|
| **Build Slots** | CI/CD isolation prevents parallel build conflicts |
| **Macros** | Reusable automation templates |
| **Enhanced Contacts** | Bidirectional with policy enforcement |
| **Products** | First-class multi-repo coordination |
| **PWA Web UI** | Installable, offline-capable |
| **Performance** | Native binary, lower memory footprint |
| **Type Safety** | Compile-time guarantees |
| **Single Binary** | No Python runtime dependency |

---

## Migration Path

For teams currently using Python `mcp_agent_mail`:

### Step 1: Install Rust Binary
```bash
# When installer exists:
curl -fsSL https://raw.githubusercontent.com/.../install.sh | bash
```

### Step 2: Stop Python Server
```bash
pkill -f "mcp_agent_mail"
```

### Step 3: Migrate Database (Optional)
Both use SQLite with compatible schema. For fresh start:
```bash
rm data/archive.db  # Rust will recreate
# Git archive is compatible - no migration needed
```

### Step 4: Update MCP Config
```json
{
  "mcpServers": {
    "agent-mail": {
      "command": "mcp-agent-mail",
      "args": ["serve-mcp"]
    }
  }
}
```

### Step 5: Verify
```bash
curl http://127.0.0.1:8765/health
bd ready --json  # Should work unchanged
```

---

## Conclusion

The Rust `mcp-agent-mail-rs` implementation is **production-ready for core functionality** and offers **additional features** (build slots, macros, products) not in the Python version.

**Blocking issues for drop-in replacement:**
1. No unified CLI tool with `serve-http` subcommand
2. No one-line installer script
3. Missing documentation for beads environment variables

**Estimated effort to close P0 gaps**: 2-3 days

**Recommendation**: Prioritize CLI unification and installer before promoting as Python replacement. The extended features (build slots, macros) provide immediate value for complex multi-agent workflows.
