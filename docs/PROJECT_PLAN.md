# MCP Agent Mail (Rust) - Project Plan

## Overview

Rust-native rewrite of [mcp_agent_mail](https://github.com/Dicklesworthstone/mcp_agent_mail) - "Gmail for coding agents". This system enables multiple AI coding agents to coordinate work on shared codebases through messaging, file reservations, and searchable archives.

**Source**: The Python original provides [28 MCP tools](https://glama.ai/mcp/servers/@Dicklesworthstone/mcp_agent_mail). Our Rust implementation tracks **39 remaining tasks** (41 total, 2 closed) organized into finer-grained work items for systematic implementation.

## Phase Summary

| Phase | Description | Status | Beads Epic |
|-------|-------------|--------|------------|
| 1 | Core Architecture | COMPLETE | `mcp-agent-mail-rs-1yw` |
| 1.5 | API Layer (Axum REST) | COMPLETE | `mcp-agent-mail-rs-cgm` |
| 2 | SvelteKit Frontend | COMPLETE | `mcp-agent-mail-rs-k43` |
| 3 | Full Feature Parity (28 MCP tools) | IN PROGRESS | `mcp-agent-mail-rs-geo` |
| 4 | MCP Protocol Integration | PLANNED | `mcp-agent-mail-rs-2m0` |
| 5 | Production Hardening | PLANNED | `mcp-agent-mail-rs-pw4` |

---

## Phase 1: Core Architecture (COMPLETE)

**Commit**: `39503c1`

### Deliverables
- Workspace structure: `lib-core`, `mcp-server`, `mcp-cli`
- Storage layer: libsql (SQLite) + git2 (Git mailbox)
- BMC pattern for Project, Agent, Message entities
- thiserror-based error handling

### Key Files
- `crates/libs/lib-core/src/model/` - Domain entities
- `crates/libs/lib-core/src/store/git_store.rs` - Git operations
- `crates/libs/lib-core/src/store/db.rs` - Database setup

---

## Phase 1.5: API Layer (COMPLETE)

**Commits**: `892610d`, `1e293e0`, `3c2af28`, `f84973c`

### Deliverables
- Axum 0.8 web server with JSON API
- REST endpoints for all core operations
- FileReservation model and BMC
- Bug fixes: column indices, datetime parsing, Path extractors

### API Endpoints
| Method | Path | Handler |
|--------|------|---------|
| GET | `/api/health` | `health_check` |
| POST | `/api/project/ensure` | `ensure_project` |
| POST | `/api/agent/register` | `register_agent` |
| POST | `/api/message/send` | `send_message` |
| POST | `/api/inbox` | `list_inbox` |
| GET | `/api/projects` | `list_all_projects` |
| GET | `/api/projects/:slug/agents` | `list_all_agents_for_project` |
| GET | `/api/messages/:id` | `get_message` |
| POST | `/api/file_reservations/paths` | `file_reservation_paths` |

---

## Phase 2: SvelteKit Frontend (COMPLETE)

**Commits**: `76a8c00`, `049e133`

### Deliverables
- SvelteKit 2 + Svelte 5 + TailwindCSS + Bun
- Material Design 3 theming
- Static adapter for Rust binary embedding

### Pages
- `/` - Projects list with search
- `/agents` - Agents list with project filter
- `/inbox` - Inbox with project/agent selector
- `/inbox/[id]` - Message thread view
- `ComposeMessage.svelte` - Compose modal

---

## Phase 3: Full Feature Parity (IN PROGRESS)

**Epic**: `mcp-agent-mail-rs-geo` (41 child tasks, 2 closed, 39 open)

Implement all 28 MCP tools from the [Python original](https://glama.ai/mcp/servers/@Dicklesworthstone/mcp_agent_mail) organized by cluster:

### Python MCP Tools (28 total)

| Category | Tools | Count |
|----------|-------|-------|
| **Messaging** | send_message, fetch_inbox, acknowledge_message, reply_message, search_messages, summarize_thread | 6 |
| **Identity** | register_agent, whois, list_agents, set_contact_policy | 4 |
| **File Reservations** | file_reservation_paths, release_file_reservations, force_release_file_reservation | 3 |
| **Contacts** | request_contact, respond_contact, list_contacts, macro_contact_handshake | 4 |
| **Project/Product** | ensure_project, ensure_product, products_link, search_messages_product | 4 |
| **Build Slots** | acquire_build_slot, renew_build_slot, release_build_slot | 3 |
| **Macros** | macro_start_session, macro_prepare_thread, macro_file_reservation_cycle | 3 |
| **Utility** | health_check | 1 |

### Beads Task Breakdown

#### Already Implemented (Phase 1.5)
- `geo.1` ✅ FileReservation model and BMC
- `geo.2` ✅ file_reservation_paths tool

#### P1 - Critical Path (11 open)

| ID | Tool | Python Equivalent |
|----|------|-------------------|
| geo.3 | `whois` | whois |
| geo.4 | `create_agent_identity` | register_agent (name generation) |
| geo.5 | `reply_message` | reply_message |
| geo.12 | `release_file_reservation` | release_file_reservations |
| geo.13 | `force_release_reservation` | force_release_file_reservation |
| geo.14 | `renew_file_reservation` | (TTL extension) |
| geo.18 | `search_messages` | search_messages |
| geo.34 | `list_file_reservations` | (list active) |
| geo.35 | `get_project_info` | ensure_project (read) |
| geo.36 | `get_agent_profile` | whois (extended) |
| geo.39 | `get_thread` | (thread messages) |

#### P2 - Standard Priority (22 open)

| ID | Tool | Python Equivalent |
|----|------|-------------------|
| geo.6 | `mark_message_read` | (read tracking) |
| geo.7 | `acknowledge_message` | acknowledge_message |
| geo.8 | `request_contact` | request_contact |
| geo.9 | `respond_contact` | respond_contact |
| geo.10 | `list_contacts` | list_contacts |
| geo.11 | `set_contact_policy` | set_contact_policy |
| geo.15 | `acquire_build_slot` | acquire_build_slot |
| geo.16 | `renew_build_slot` | renew_build_slot |
| geo.17 | `release_build_slot` | release_build_slot |
| geo.19 | `summarize_thread` | summarize_thread |
| geo.20 | `summarize_threads` | (batch variant) |
| geo.21 | `invoke_macro` | macro_* |
| geo.22 | `list_macros` | (macro registry) |
| geo.23 | `register_macro` | (macro creation) |
| geo.24 | `unregister_macro` | (macro deletion) |
| geo.25 | `install_precommit_guard` | (guard.py) |
| geo.26 | `uninstall_precommit_guard` | (guard.py) |
| geo.32 | `add_attachment` | send_message (attachments) |
| geo.33 | `get_attachment` | (attachment retrieval) |
| geo.37 | `update_agent_profile` | (profile update) |
| geo.38 | `send_overseer_message` | (human operator) |
| geo.40 | `list_threads` | (thread listing) |

#### P3 - Product Bus & Export (6 open)

| ID | Tool | Python Equivalent |
|----|------|-------------------|
| geo.27 | `ensure_product` | ensure_product |
| geo.28 | `link_project_to_product` | products_link |
| geo.29 | `unlink_project_from_product` | (unlink) |
| geo.30 | `list_products` | (product listing) |
| geo.31 | `product_inbox` | search_messages_product |
| geo.41 | `export_mailbox` | share.py (static export)

---

## Phase 4: MCP Protocol Integration (PLANNED)

**Epic**: `mcp-agent-mail-rs-2m0`

### Planned Work
- Integrate `mcp-protocol-sdk` crate
- Expose API as MCP-compliant server
- Tool registration and discovery
- JSON-RPC 2.0 transport layer

---

## Phase 5: Production Hardening (PLANNED)

**Epic**: `mcp-agent-mail-rs-pw4`

### Planned Work
- JWT/bearer token authentication
- Rate limiting and abuse prevention
- Structured logging with tracing
- Prometheus metrics endpoint
- Docker multi-stage build
- CI/CD pipeline (GitHub Actions)
- Load testing and benchmarks

---

## Development Workflow

```bash
# View all issues
bd list --all

# View Phase 3 progress
bd show geo

# Start work on a task
bd update geo.18 --status in_progress

# Complete a task
bd close geo.18

# Build and test
cargo build --release
cargo test
```

---

## Key Design Decisions

### 1. Git-Backed Storage
All messages stored in both SQLite (fast queries) and Git (audit trail):
```
projects/{slug}/messages/YYYY/MM/{timestamp}__{subject}__{id}.md
```

### 2. BMC Pattern
Backend Model Controller separates:
- `{Entity}` - Database row struct
- `{Entity}ForCreate` - Creation input
- `{Entity}Bmc` - Business logic

### 3. Advisory File Reservations
Reservations are advisory (not enforced by kernel):
- Agents signal intent via reservations
- Pre-commit hook checks for conflicts
- Emergency bypass via `AGENT_MAIL_BYPASS=1`

### 4. Memorable Agent Names
Adjective+Noun pattern (BlueMountain, GreenCastle) aids:
- Human readability in logs
- Agent identification across sessions
- Audit trail comprehension

---

## Reference Materials

- [Python Original](https://github.com/Dicklesworthstone/mcp_agent_mail)
- [MCP Tools Reference](https://glama.ai/mcp/servers/@Dicklesworthstone/mcp_agent_mail) - Authoritative list of 28 MCP tools
- [Sharing Plan](https://github.com/Dicklesworthstone/mcp_agent_mail/blob/main/PLAN_TO_ENABLE_EASY_AND_SECURE_SHARING_OF_AGENT_MAILBOX.md)
- [Worktree Integration](https://github.com/Dicklesworthstone/mcp_agent_mail/blob/main/PLAN_TO_NON_DISRUPTIVELY_INTEGRATE_WITH_THE_GIT_WORKTREE_APPROACH.md)
- [Beads Issue Tracker](https://github.com/steveyegge/beads)
