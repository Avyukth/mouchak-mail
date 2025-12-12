# Gap Analysis: Rust vs Python MCP Agent Mail

> Comprehensive comparison for drop-in replacement of Python `mcp_agent_mail`.
> **Date**: 2025-12-12
> **Scope:** Backend API only (Web UI excluded per requirements)

---

## Executive Summary

| Metric | Python | Rust | Gap |
|--------|--------|------|-----|
| API Endpoints | 62+ | 47 | 76% |
| MCP Tools | 70+ | 47 | 67% |
| CLI Commands | 30+ | 0 | **BLOCKER** |
| Integration Scripts | 8 | 0 | **BLOCKER** |
| Authentication | Bearer + JWT | None | **BLOCKER** |
| MCP Resources | 5 URIs | 0 | Missing |
| Built-in Macros | 5 | 0 | Missing |

**Overall Readiness: 65% - NOT DROP-IN READY**

---

## P0: Critical Blockers (Must Fix)

### 1. CLI Binary (`mcp_agent_mail.cli`)

**Python:** 166KB comprehensive CLI
```bash
# Config management
uv run python -m mcp_agent_mail.cli config set-port 9000
uv run python -m mcp_agent_mail.cli config show

# Server control
uv run python -m mcp_agent_mail.cli server start
uv run python -m mcp_agent_mail.cli server status

# Documentation
uv run python -m mcp_agent_mail.cli docs insert-blurbs

# Health
uv run python -m mcp_agent_mail.cli health check
```

**Rust:** None

**Action Required:**
```rust
// crates/services/mcp-cli/src/main.rs
// Implement: config, server, docs, health subcommands
```

### 2. One-Line Installer (`scripts/install.sh`)

**Python:** 18KB installer script
```bash
curl -fsSL https://raw.githubusercontent.com/Dicklesworthstone/mcp_agent_mail/main/scripts/install.sh | bash -s -- --yes
```

Features:
- Installs uv if missing
- Creates Python 3.14 venv
- Runs auto-detect integration
- Starts server on port 8765
- Installs beads CLI
- Prints setup summary

**Rust:** None

**Action Required:**
- Create `scripts/install.sh` for Rust binary
- Support `--port`, `--dir`, `--token` flags
- Auto-detect and integrate with coding agents

### 3. Integration Scripts

**Python:** 8 integration scripts (7-18KB each)
```
scripts/
├── integrate_claude_code.sh      # 11KB
├── integrate_codex_cli.sh        # 6KB
├── integrate_cursor.sh           # 6KB
├── integrate_cline.sh            # 6KB
├── integrate_windsurf.sh         # 6KB
├── integrate_gemini_cli.sh       # 7KB
├── integrate_github_copilot.sh   # 9KB
├── integrate_opencode.sh         # 8KB
└── automatically_detect_all_installed_coding_agents_and_install_mcp_agent_mail_in_all.sh  # 7KB
```

**Rust:** None

**Action Required:**
- Port all 8 integration scripts
- Update config paths for Rust binary
- Test with each coding agent

### 4. Authentication

**Python:**
```python
# Bearer token (static)
HTTP_BEARER_TOKEN=<hex>

# JWT/JWKS (dynamic)
HTTP_AUTH_MODE=jwt
HTTP_JWKS_URL=<url>
```

**Rust:** No authentication layer

**Action Required:**
```rust
// Add middleware for:
// 1. Static bearer token validation
// 2. JWT validation with JWKS discovery
// 3. Localhost bypass option
```

### 5. Environment Variable Documentation

**Python:** `.env.example` with 30+ variables
```bash
# Core
HTTP_HOST=127.0.0.1
HTTP_PORT=8765
HTTP_BEARER_TOKEN=

# Auth
HTTP_AUTH_MODE=bearer
HTTP_JWKS_URL=
HTTP_ALLOW_LOCALHOST_UNAUTHENTICATED=true

# Database
SQLITE_PATH=./data/mcp_agent_mail.db

# Git
GIT_REPO_PATH=./data/archive

# LLM (optional)
LLM_ENABLED=false
OPENAI_API_KEY=
```

**Rust:** None

**Action Required:**
- Create `.env.example`
- Document all supported variables in README

### 6. MCP STDIO Mode

**Python:**
```bash
# HTTP mode
uv run python -m mcp_agent_mail.http

# STDIO mode (for direct MCP client integration)
uv run python -m mcp_agent_mail
```

**Rust:** HTTP only (`mcp-server`), STDIO incomplete

**Action Required:**
- `mcp-stdio` crate exists but needs full tool implementation
- Must support same tool signatures as HTTP API

---

## P1: Important Features

### 1. Outbox Endpoint

**Python:**
```python
@mcp.tool()
async def fetch_outbox(project_key: str, agent_name: str, limit: int = 20):
    """Fetch messages sent BY this agent"""
```

**Rust:** Missing

**Action Required:**
```rust
// api.rs
.route("/api/outbox", post(tools::list_outbox))
```

### 2. MCP Resources

**Python:** 5 resource URIs
```python
resource://inbox/{agent}?project=<path>&limit=20
resource://outbox/{agent}?project=<path>&limit=20
resource://thread/{id}?project=<path>&include_bodies=true
resource://agents?project=<path>
resource://file_reservations?project=<path>
```

**Rust:** None

**Action Required:**
- Implement resource handler in rmcp integration
- Map URIs to existing API calls

### 3. Built-in Macros

**Python:** 5 pre-registered macros
```python
macro_start_session      # Register agent + check inbox
macro_prepare_thread     # Create thread + reserve files
macro_file_reservation_cycle  # Reserve, work, release
macro_contact_handshake  # Cross-project contact setup
macro_broadcast_message  # Send to multiple agents
```

**Rust:** Macro storage exists but no built-ins

**Action Required:**
- Pre-register macros in DB migrations
- Or create on first server start

### 4. Capabilities/RBAC

**Python:**
```python
# deploy/capabilities/agent_capabilities.yaml
capabilities:
  architect:
    - send_message
    - file_reservation_paths
    - summarize_thread
  worker:
    - send_message
    - fetch_inbox
    - acknowledge_message
```

**Rust:** No RBAC layer

**Action Required:**
- Add capabilities config file support
- Implement middleware for tool-level RBAC

### 5. CC/BCC Recipients

**Python:**
```python
@mcp.tool()
async def send_message(
    to_agent_names: list[str],
    cc_agent_names: list[str] = [],
    bcc_agent_names: list[str] = [],
    ...
):
```

**Rust:** Only `recipient_names` (equivalent to `to`)

**Action Required:**
```rust
pub struct SendMessagePayload {
    pub recipient_names: Vec<String>,  // to
    pub cc_names: Option<Vec<String>>,
    pub bcc_names: Option<Vec<String>>,
}
```

### 6. Project Siblings (AI-Powered)

**Python:**
```python
async def refresh_project_sibling_suggestions():
    """
    AI-powered detection of related projects
    (e.g., frontend/backend of same product)
    """
```

**Rust:** Model exists but no implementation

**Action Required:**
- Implement heuristic scoring
- Add LLM pass for confirmation (optional)

### 7. Git Archive Integration

**Python:** Full markdown archive
```
data/archive/
└── projects/
    └── my-project/
        ├── agents/
        │   └── profile.json
        ├── mailboxes/
        │   ├── BlueMountain/
        │   │   ├── inbox/
        │   │   └── outbox/
        ├── messages/
        │   └── 2025/01/
        │       └── msg_123.md
        └── file_reservations/
            └── sha1.json
```

**Rust:** Partial (attachments only)

**Action Required:**
- Write messages to Git as markdown
- Maintain inbox/outbox symlinks
- Store file reservations as JSON

---

## P2: Nice to Have

| Feature | Python | Rust | Priority |
|---------|--------|------|----------|
| Systemd service files | ✅ | ❌ | P2 |
| Docker deployment | ✅ | ❌ | P2 |
| Rich terminal logger | ✅ 33KB | ❌ | P2 |
| LLM summarization | ✅ Real | ⚠️ Stubbed | P2 |
| Export module | ✅ 5KB | ❌ | P2 |
| Share module | ✅ 86KB | ❌ | P2 |
| gunicorn config | ✅ | ❌ (tokio native) | P2 |

---

## API Endpoint Comparison

### ✅ Implemented in Rust

| Category | Endpoint | Status |
|----------|----------|--------|
| Health | `/api/health` | ✅ |
| Projects | `/api/project/ensure` | ✅ |
| | `/api/projects` | ✅ |
| | `/api/projects/{slug}/agents` | ✅ |
| | `/api/project/info` | ✅ |
| Agents | `/api/agent/register` | ✅ |
| | `/api/agent/whois` | ✅ |
| | `/api/agent/create_identity` | ✅ |
| | `/api/agent/profile` | ✅ |
| | `/api/agent/profile/update` | ✅ |
| Messages | `/api/message/send` | ✅ |
| | `/api/message/reply` | ✅ |
| | `/api/message/read` | ✅ |
| | `/api/message/acknowledge` | ✅ |
| | `/api/messages/search` | ✅ |
| | `/api/inbox` | ✅ |
| | `/api/messages/{id}` | ✅ |
| Threads | `/api/thread` | ✅ |
| | `/api/threads` | ✅ |
| | `/api/thread/summarize` | ✅ (stubbed) |
| | `/api/threads/summarize` | ✅ |
| File Reservations | `/api/file_reservations/paths` | ✅ |
| | `/api/file_reservations/list` | ✅ |
| | `/api/file_reservations/release` | ✅ |
| | `/api/file_reservations/force_release` | ✅ |
| | `/api/file_reservations/renew` | ✅ |
| Contacts | `/api/contacts/request` | ✅ |
| | `/api/contacts/respond` | ✅ |
| | `/api/contacts/list` | ✅ |
| | `/api/contacts/policy` | ✅ |
| Build Slots | `/api/build_slots/acquire` | ✅ |
| | `/api/build_slots/renew` | ✅ |
| | `/api/build_slots/release` | ✅ |
| Overseer | `/api/overseer/send` | ✅ |
| Macros | `/api/macros/list` | ✅ |
| | `/api/macros/register` | ✅ |
| | `/api/macros/unregister` | ✅ |
| | `/api/macros/invoke` | ✅ |
| Setup | `/api/setup/install_guard` | ✅ |
| | `/api/setup/uninstall_guard` | ✅ |
| Attachments | `/api/attachments/add` | ✅ |
| | `/api/attachments/get` | ✅ |

### ❌ Missing in Rust

| Category | Endpoint | Priority |
|----------|----------|----------|
| Messages | `/api/outbox` | P1 |
| | `/api/message/cc` | P1 |
| | `/api/message/bcc` | P1 |
| Project Siblings | `/api/projects/siblings` | P1 |
| | `/api/projects/siblings/refresh` | P1 |
| | `/api/projects/siblings/confirm` | P1 |
| Capabilities | `/api/capabilities` | P1 |
| Tool Metrics | `/api/metrics/tools` | P2 |
| Recent Activity | `/api/recent` | P2 |

---

## Database Schema Comparison

### ✅ Matching Tables

| Table | Python | Rust | Match |
|-------|--------|------|-------|
| projects | ✅ | ✅ | ✅ |
| agents | ✅ | ✅ | ✅ |
| messages | ✅ | ✅ | ⚠️ Missing cc/bcc |
| message_recipients | ✅ | ✅ | ⚠️ Missing type column |
| file_reservations | ✅ | ✅ | ✅ |
| agent_links | ✅ | ✅ | ✅ |
| build_slots | ✅ | ✅ | ✅ |
| macro_defs | ✅ | ✅ | ✅ |
| overseer_messages | ✅ | ✅ | ✅ |
| project_sibling_suggestions | ✅ | ✅ | ✅ |
| products | ✅ | ✅ | ✅ |

### ❌ Missing Tables/Columns

```sql
-- message_recipients needs recipient_type
ALTER TABLE message_recipients ADD COLUMN recipient_type TEXT DEFAULT 'to';

-- Add capabilities table
CREATE TABLE agent_capabilities (
    id INTEGER PRIMARY KEY,
    agent_id INTEGER REFERENCES agents(id),
    capability TEXT NOT NULL,
    granted_at TEXT NOT NULL
);
```

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

## Action Plan for Drop-In Replacement

### Week 1: P0 Blockers

| Task | Effort | Beads ID |
|------|--------|----------|
| CLI binary (config, server, health) | 3d | `577.1` |
| Authentication middleware (bearer) | 2d | `577.11` |
| `.env.example` + docs | 0.5d | `1aj` |
| MCP STDIO mode completion | 2d | `mzj` |

### Week 2: P0 Continued + P1

| Task | Effort | Beads ID |
|------|--------|----------|
| Installer script | 1d | `577.2` |
| Integration scripts (8) | 3d | `dlf` |
| Outbox endpoint | 0.5d | `ctb` |
| CC/BCC support | 1d | `fw1` |
| MCP Resources (5) | 2d | `if9` |

### Week 3: P1 Completion

| Task | Effort | Beads ID |
|------|--------|----------|
| Built-in macros | 1d | `4mw` |
| Capabilities/RBAC | 2d | `rkm` |
| Git archive integration | 2d | `azc` |
| Project siblings | 1d | `y58` |
| JWT/JWKS auth | 1d | `q4u` |
| DB: recipient_type column | 0.5d | `yyh` |
| DB: agent_capabilities table | 0.5d | `t0f` |

---

## Testing Checklist

### Compatibility Tests

```bash
# Same API contract
curl -X POST http://localhost:8765/api/project/ensure \
  -H "Content-Type: application/json" \
  -d '{"human_key": "/path/to/project"}'

# Same response format
{
  "project_id": 1,
  "slug": "project"
}
```

### Integration Tests

- [ ] Claude Code integration works
- [ ] Codex CLI integration works
- [ ] Cursor integration works
- [ ] beads integration works
- [ ] Pre-commit guard works
- [ ] File reservation conflicts detected

### Migration Test

```bash
# Export from Python
python -m mcp_agent_mail.cli export --format json > backup.json

# Import to Rust
mcp-cli import backup.json

# Verify data integrity
mcp-cli health check
```

---

## Conclusion

**Current State:** 65% compatible, NOT ready for drop-in replacement

**Critical Path:**
1. CLI binary (blocks all user workflows)
2. Authentication (blocks production use)
3. Integration scripts (blocks agent setup)
4. STDIO mode (blocks direct MCP integration)

**Estimated Time to Parity:** 3 weeks (focused effort)

**Recommendation:** Complete P0 blockers before any P1/P2 work.
