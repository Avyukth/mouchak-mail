# AGENTS.md — Universal Operating Manual for AI Coding Agents

> **Quick Start**: Run `cm context "<your task>"` before starting work. Run `bd ready` to find unblocked issues. Run `bd sync` before ending your session.

This document provides standardized instructions for ANY AI coding agent (Claude, Gemini, GPT, Codex, etc.) working in this codebase. It is divided into:

1. **Layer 0**: Inviolable safety rules (NEVER break these)
2. **Layer 1**: Universal tooling (works across all projects)
3. **Layer 2**: Session workflow (how to start, work, and end sessions)
4. **Layer 3**: Project-specific configuration (filled in per-project)
5. **Layer 4**: Language/stack-specific instructions (filled in per-project)

---

## ⛔ LAYER 0: INVIOLABLE SAFETY RULES

These rules are ABSOLUTE and apply to ALL agents in ALL contexts.

### Rule 1: NO FILE DELETION WITHOUT EXPLICIT PERMISSION

```
YOU ARE NEVER ALLOWED TO DELETE A FILE WITHOUT EXPRESS WRITTEN PERMISSION.
```

- Applies to ALL files: user files, test files, temporary files, files you created
- You must ASK and RECEIVE clear written permission before ANY deletion
- "I think it's safe" is NEVER acceptable justification
- This includes: `rm`, `unlink`, file system APIs, git clean operations

### Rule 2: NO DESTRUCTIVE GIT/FILESYSTEM COMMANDS

**Forbidden commands** (unless user provides exact command AND acknowledges consequences in same message):

| Command | Risk |
|---------|------|
| `git reset --hard` | Destroys uncommitted work |
| `git clean -fd` | Deletes untracked files |
| `rm -rf` | Recursive deletion |
| `git push --force` | Overwrites remote history |
| Any command that deletes/overwrites code or data | Potential data loss |

**Mandatory protocol for ANY destructive operation:**

1. **Safer alternatives first**: Use `git status`, `git diff`, `git stash`, or copy to backups
2. **Explicit plan**: Restate the command verbatim + list exactly what will be affected
3. **Wait for confirmation**: Do NOT proceed without explicit user approval
4. **Document**: Record user authorization text, command run, and execution time
5. **Refuse if ambiguous**: If ANY uncertainty remains, refuse and escalate

### Rule 3: PROTECT CONFIGURATION FILES

- **`.env` files**: NEVER overwrite—they contain secrets and local configuration
- **Lock files**: Do not delete `package-lock.json`, `Cargo.lock`, `go.sum`, etc.
- **Database files**: Never delete `.db`, `.sqlite`, or data files without explicit permission

---

## 🧠 LAYER 1: UNIVERSAL TOOLING

These tools work across ALL projects. Learn them once, use them everywhere.

### Quick Reference: Tool Selection

| Task | Tool | Command |
|------|------|---------|
| **Start new task** | cm | `cm context "<task>"` |
| **Find ready issues** | bd | `bd ready --json` |
| **Search past sessions** | cass | `cass search "query" --robot` |
| **Graph analysis of issues** | bv | `bv --robot-insights` |
| **Bug scan before commit** | ubs | `ubs $(git diff --name-only --cached)` |
| **Structural code search** | ast-grep | `ast-grep run -l <lang> -p 'pattern'` |
| **Text search** | ripgrep | `rg "pattern"` |
| **Multi-agent coordination** | MCP Agent Mail | `file_reservation_paths(...)` |

---

### 📚 cm (CASS Memory System) — Context Hydration

**What it does**: Provides "procedural memory" across coding sessions. Before starting ANY non-trivial task, hydrate your context.

**Primary command**:
```bash
cm context "<your task description>"
```

This returns:
- Relevant rules from the project playbook (scored by task relevance)
- Anti-patterns to avoid (things that caused problems before)
- Historical context from past sessions

| Command | Purpose |
|---------|---------|
| `cm context "<task>"` | **START HERE** — Get context for your task |
| `cm doctor` | System health check (exit 0 = healthy) |
| `cm init` | Initialize playbook for new project |
| `cm mark <id> --helpful` | Positive feedback on a rule |
| `cm mark <id> --harmful` | Negative feedback on a rule |
| `cm stats` | Playbook health metrics |
| `cm similar "<query>"` | Find similar rules |
| `cm top` | Top N rules by effectiveness score |
| `cm forget <id>` | Deprecate a harmful rule |
| `cm why <id>` | Explain reasoning behind a rule |
| `cm diary` | Record session as diary entry |
| `cm project` | Export playbook to AGENTS.md/CLAUDE.md |

**Output conventions**: stdout = data, stderr = diagnostics. Exit 0 = success.

---

### 🔎 cass (Cross-Agent Session Search) — History Search

**What it does**: Indexes conversation histories from ALL AI coding agents (Claude, Codex, Cursor, Gemini, Aider, ChatGPT, etc.) into a unified, searchable archive.

**Before solving a problem from scratch, check if ANY agent already solved something similar.**

⚠️ **CRITICAL**: NEVER run bare `cass` — it launches an interactive TUI. Always use `--robot` or `--json`.

#### Pre-Flight Health Check

```bash
cass health --json
# Exit 0 = healthy (proceed with searches)
# Exit 1 = unhealthy (run: cass index --full)
```

#### Core Commands

```bash
# Search across all agent histories
cass search "query" --robot --limit 5
cass search "query" --robot --agent claude --workspace /path

# View specific result from search output
cass view /path/to/session.jsonl -n 42 --json

# Expand context around a line (like grep -C)
cass expand /path/to/session.jsonl -n 42 -C 3 --json

# Index management
cass index --full          # Build/rebuild index
cass state --json          # View index state

# Discovery
cass capabilities --json   # Available features
cass robot-docs guide      # LLM-optimized documentation
cass timeline --days 7 --robot
```

#### Key Flags

| Flag | Purpose |
|------|---------|
| `--robot` / `--json` | **Required for automation** — structured output |
| `--limit N` | Cap number of results |
| `--agent NAME` | Filter: claude, codex, cursor, gemini, aider, chatgpt, opencode, amp, cline, pi-agent |
| `--days N` | Limit to recent N days |
| `--workspace PATH` | Restrict to specific directory |
| `-C N` | Context lines for expand |
| `--fields minimal` | Reduced payload (source_path, line_number, agent only) |

#### Exit Codes

| Code | Meaning | Action |
|------|---------|--------|
| 0 | Success | Proceed |
| 1 | Unhealthy | Run `cass index --full` |
| 2 | Usage error | Fix syntax |
| 3 | Missing index | Run `cass index` |
| 9 | Unknown error | Retry or investigate |

#### Auto-Correction

cass auto-corrects common syntax mistakes and proceeds:
- `-robot` → `--robot` (single-dash to double-dash)
- `find "query"` → `search "query"` (alias resolution)
- `--Robot` → `--robot` (case normalization)
- Flag position errors → hoisted automatically

Corrections appear as JSON on stderr; command still executes successfully.

---

### 📋 bd (Beads) — Issue Tracking

**What it does**: Git-native issue tracking designed for AI-supervised workflows. Dependencies, priorities, and atomic operations.

**IMPORTANT**: Use bd for ALL issue tracking. Do NOT use markdown TODOs, task lists, or external trackers.

#### Essential Workflow

```bash
# 1. Find ready work (no blockers)
bd ready --json

# 2. Create issues (ALWAYS include description)
bd create "Issue title" \
  --description="Detailed context about what and why" \
  -t bug|feature|task|epic|chore \
  -p 0-4 \
  --json

# 3. Link discovered work to parent
bd create "Found bug during work" \
  --description="Details about the bug" \
  -t bug -p 1 \
  --deps discovered-from:<parent-id> \
  --json

# 4. Update status
bd update <id> --status in_progress --json

# 5. Complete work
bd close <id> --reason "Completed" --json

# 6. CRITICAL: Sync at end of session
bd sync
```

#### Issue Types

| Type | Use For |
|------|---------|
| `bug` | Something broken that needs fixing |
| `feature` | New functionality |
| `task` | Work items (tests, docs, refactoring) |
| `epic` | Large feature with subtasks |
| `chore` | Maintenance (dependencies, tooling) |

#### Priorities

| Priority | Meaning | Examples |
|----------|---------|----------|
| 0 | Critical | Security, data loss, broken builds |
| 1 | High | Major features, important bugs |
| 2 | Medium | Nice-to-have features, minor bugs |
| 3 | Low | Polish, optimization |
| 4 | Backlog | Future ideas |

#### Dependency Types

| Type | Purpose | Affects Ready Queue? |
|------|---------|---------------------|
| `blocks` | Hard dependency (X blocks Y) | Yes |
| `related` | Soft relationship | No |
| `parent-child` | Epic/subtask relationship | No |
| `discovered-from` | Track work discovered during other work | No |

#### Key Commands

| Command | Purpose |
|---------|---------|
| `bd ready --json` | Show unblocked issues |
| `bd stale --days 30 --json` | Find forgotten issues |
| `bd list --status open --json` | All open issues |
| `bd show <id> --json` | Issue details |
| `bd dep tree <id>` | Visualize dependency tree |
| `bd duplicates --auto-merge` | Find and merge duplicates |
| `bd sync` | Force immediate export/commit/push |
| `bd hooks install` | Install git hooks for auto-sync |

#### ALWAYS Include Descriptions

Issues without descriptions lack context for future work:

```bash
# ❌ BAD - No context
bd create "Fix auth bug" -t bug -p 1 --json

# ✅ GOOD - Full context
bd create "Fix auth bug in login handler" \
  --description="Login fails with 500 error when password contains special characters. Found while testing GH#123. Stack trace shows unescaped SQL in auth/login.go:45." \
  -t bug -p 1 \
  --deps discovered-from:bd-abc \
  --json
```

---

### 📊 bv (Beads Visualizer) — Graph Analysis

**What it does**: Precomputes dependency metrics (PageRank, critical path, cycles) for issue triage. Use instead of manually parsing JSONL.

```bash
bv --robot-help           # AI-facing commands
bv --robot-insights       # Graph metrics (PageRank, critical path, cycles)
bv --robot-plan           # Execution plan with parallel tracks
bv --robot-priority       # Priority recommendations with reasoning
bv --robot-recipes        # List available filter recipes
bv --robot-diff --diff-since <commit|date>  # Changes since point
```

---

### 🔍 ubs (Ultimate Bug Scanner) — Pre-Commit Validation

**What it does**: Static analysis across multiple languages. Run before EVERY commit.

```bash
# Specific files (fast, <1s) — PREFERRED
ubs file.rs file2.rs

# Staged files — before commit
ubs $(git diff --name-only --cached)

# With language filter
ubs --only=rust,toml src/

# Whole project
ubs .
```

**Exit 0 = safe to commit. Exit >0 = fix issues first.**

#### Severity Tiers

| Tier | Action | Examples |
|------|--------|----------|
| **Critical** | Always fix | Memory safety, use-after-free, data races, SQL injection |
| **Important** | Fix for production | Unwrap panics, resource leaks |
| **Contextual** | Use judgment | TODO/FIXME, println! debugging |

---

### 🔧 ast-grep vs ripgrep — Code Search Decision Tree

| Need | Tool | Example |
|------|------|---------|
| Structural match (ignores comments/strings) | ast-grep | `ast-grep run -l Rust -p 'fn $NAME($$$) -> $RET'` |
| Safe codemod/refactor | ast-grep | `ast-grep run -l Rust -p '$E.unwrap()' -r '$E.expect("msg")' -U` |
| Fast text search | ripgrep | `rg -n 'TODO' -t rust` |
| Find files, then precise match | Both | `rg -l 'unwrap\(' -t rust \| xargs ast-grep run ...` |

**Rule of thumb**:
- Need **correctness** or will **apply changes** → ast-grep
- Need **speed** or just **hunting text** → ripgrep

---

### 🤝 MCP Agent Mail — Multi-Agent Coordination

**What it does**: Async coordination for multi-agent workflows via file reservations and messaging.

#### When to Use

- ✅ Multiple agents working concurrently on same repo
- ✅ Need to prevent file conflicts
- ✅ Real-time coordination required
- ✅ Single agent workflows (works the same)

#### Setup (Same Repo)

```bash
# 1. Register identity
ensure_project → register_agent(project_key=<abs-path>, agent_name=<name>)

# 2. Reserve files before editing (prevents conflicts)
file_reservation_paths(project_key, agent_name, ["src/**"], ttl_seconds=3600, exclusive=true)

# 3. Communicate via threads
send_message(..., thread_id="FEAT-123")
fetch_inbox → acknowledge_message
```

#### Resources

- `resource://inbox/{Agent}?project=<path>&limit=20`
- `resource://thread/{id}?project=<path>&include_bodies=true`

**Tip**: Set `AGENT_NAME` env var for pre-commit guard to block conflicting commits.

---

## 🔄 LAYER 2: SESSION WORKFLOW

### Starting a Session

```bash
# 1. Hydrate context for your task
cm context "<what you're working on>"

# 2. Check system health
cm doctor
cass health --json

# 3. Find ready work
bd ready --json

# 4. View issue details
bd show <id> --json

# 5. Claim work (add notes, don't change status unless you're an automated executor)
bd update <id> --notes "Starting work in this session"
```

### During Work

```bash
# Search past sessions for similar problems
cass search "error message or concept" --robot --limit 5

# Create discovered issues linked to current work
bd create "Found bug" --description="Details" -p 1 --deps discovered-from:<current-id> --json

# Update progress
bd update <id> --notes "Completed X, working on Y"
```

### Planning Work with Dependencies

**⚠️ COGNITIVE TRAP**: Temporal language ("Phase 1", "Step 1", "first") inverts dependency direction.

```bash
# ❌ WRONG - temporal thinking
bd create "Phase 1: Create layout" ...
bd create "Phase 2: Add rendering" ...
bd dep add phase1 phase2  # WRONG! Says phase1 depends on phase2

# ✅ RIGHT - requirement thinking ("X needs Y")
bd create "Create layout" ...
bd create "Add rendering" ...
bd dep add rendering layout  # rendering NEEDS layout
```

**Verification**: Run `bd blocked` — tasks should be blocked by prerequisites, not dependents.

### Ending a Session ("Landing the Plane")

**MANDATORY WORKFLOW — Complete ALL steps:**

#### 1. File Remaining Work

```bash
bd create "Follow-up task discovered" \
  --description="Context about what needs doing" \
  -t task -p 2 --json
```

#### 2. Run Quality Gates (if code changed)

```bash
# Run tests (project-specific command)
# Run linters (project-specific command)
# File P0 issues for any failures
```

#### 3. Update Issues

```bash
bd close <id> --reason "Completed all acceptance criteria" --json
bd update <other-id> --notes "Partial progress, needs X" --json
```

#### 4. PUSH TO REMOTE — NON-NEGOTIABLE

```bash
# Pull first to catch remote changes
git pull --rebase

# If conflicts in .beads/issues.jsonl:
#   git checkout --theirs .beads/issues.jsonl
#   bd import -i .beads/issues.jsonl

# Sync the database (exports to JSONL, commits)
bd sync

# MANDATORY: Push everything
git push

# VERIFY: Must show "up to date with origin"
git status
```

**CRITICAL RULES**:
- The plane has NOT landed until `git push` completes successfully
- NEVER stop before `git push` — that leaves work stranded
- NEVER say "ready to push when you are" — YOU must push
- If push fails, resolve and retry until it succeeds

#### 5. Clean Up Git State

```bash
git stash clear                # Remove old stashes
git remote prune origin        # Clean up deleted remote branches
```

#### 6. Verify Clean State

```bash
git status  # Should show clean working tree, up to date with origin
```

#### 7. Provide Follow-Up Prompt

Give the user a prompt for the next session:

```
Continue work on <id>: [issue title]

Context: [1-2 sentences about what's done and what's next]
```

---

## 🎯 LAYER 3: PROJECT-SPECIFIC CONFIGURATION

### Project Overview

**MCP Agent Mail** is a production-grade multi-agent messaging system in Rust — "Gmail for coding agents". It provides asynchronous coordination for AI coding agents via messaging, file reservations, and build slot management.

**Performance**: 44.6x higher throughput than Python reference (15,200 req/s vs 341 req/s).

**Repository**: https://github.com/Avyukth/mcp-agent-mail-rs

### Repository Structure

```
mcp-agent-mail-rs/
├── crates/
│   ├── libs/                    # Library crates
│   │   ├── lib-common/          # Config (12-factor), utilities
│   │   ├── lib-core/            # Domain logic (BMC pattern)
│   │   │   ├── model/           # Entity + BMC controllers
│   │   │   │   ├── agent.rs     # AgentBmc
│   │   │   │   ├── message.rs   # MessageBmc
│   │   │   │   ├── project.rs   # ProjectBmc
│   │   │   │   └── ...
│   │   │   ├── store/           # Database, Git archive
│   │   │   └── error.rs         # Domain errors (thiserror)
│   │   ├── lib-mcp/             # MCP tools (50+)
│   │   │   └── tools.rs         # AgentMailService + JSON schemas
│   │   └── lib-server/          # HTTP layer (Axum 0.8)
│   │       ├── api/             # REST handlers
│   │       ├── auth.rs          # Bearer/JWT auth
│   │       └── ratelimit.rs     # Token bucket (100 req/min)
│   └── services/                # Binary crates
│       ├── mcp-agent-mail/      # Unified CLI (serve, migrate)
│       ├── mcp-server/          # HTTP server (REST + MCP SSE)
│       ├── mcp-stdio/           # STDIO MCP (Claude Desktop)
│       ├── mcp-cli/             # Testing CLI
│       └── web-ui-leptos/       # Leptos WASM frontend
├── migrations/                  # SQL migrations (auto-run)
├── benches/                     # Performance benchmarks
├── scripts/integrations/        # Claude, Cline, Cursor configs
├── .beads/                      # Issue tracker
└── docs/                        # Architecture documentation
```

### Project-Specific Commands

| Command | Purpose |
|---------|---------|
| `make dev-api` | Start API server on :8765 |
| `make dev` | Run API + Web UI together |
| `make test` | Run all tests |
| `make lint` | Run clippy |
| `cargo run -p mcp-agent-mail --release -- serve` | Production server |
| `cargo run -p mcp-stdio -- serve` | MCP STDIO mode |

### Environment Variables

Key variables (see `.env.example` for all 35+):

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | 8765 | HTTP server port |
| `RUST_LOG` | info | Log level filter |
| `SQLITE_PATH` | ./data/mcp_agent_mail.db | Database file |
| `GIT_REPO_PATH` | ./data/archive | Git archive path |
| `HTTP_AUTH_MODE` | none | none, bearer, jwt |
| `HTTP_BEARER_TOKEN` | — | Token for bearer auth |
| `LLM_ENABLED` | false | Enable thread summarization |

### Git Workflow

- **Main branch**: `main`
- **Feature branches**: `feature/<id>` (for worktree isolation)
- **Worktree location**: `.sandboxes/agent-<id>/`
- Always sync `.beads/issues.jsonl` with code changes

#### Multi-Agent Worktree Isolation

Each agent works in an isolated git worktree:

```bash
# Create worktree for task
git worktree add .sandboxes/agent-<id> feature/<id>
cd .sandboxes/agent-<id>

# On success: merge to main
cd ../..
git checkout main
git merge feature/<id>
git worktree remove .sandboxes/agent-<id>
git branch -d feature/<id>

# On failure (WITH PERMISSION ONLY)
git worktree remove --force .sandboxes/agent-<id>
git branch -D feature/<id>
```

### Quality Gates

```bash
# All must pass before commit:
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test -p lib-core --test integration -- --test-threads=1
```

### Active Work Areas

Current epics (check `bd ready` for latest):

- **mcp-agent-mail-rs-6et**: GitHub Binary Release v0.1.0
- **mcp-agent-mail-rs-2ci**: Pre-commit Guard MCP Integration
- **mcp-agent-mail-rs-7rh**: Workflow Macros Completion

### MCP Tools Reference (50+)

| Category | Tools |
|----------|-------|
| **Project** | ensure_project, get_project_info, list_project_siblings |
| **Agent** | register_agent, create_agent_identity, update_agent_profile, whois, list_agents |
| **Messaging** | send_message, reply_message, fetch_inbox, list_outbox, get_message, mark_message_read, acknowledge_message |
| **Threads** | list_threads, get_thread, summarize_thread, summarize_threads |
| **Search** | search_messages |
| **Files** | file_reservation_paths, list_file_reservations, release_file_reservation, force_release_reservation, renew_file_reservation |
| **Build** | acquire_build_slot, renew_build_slot, release_build_slot |
| **Contacts** | request_contact, respond_contact, list_contacts, set_contact_policy |
| **Macros** | list_macros, register_macro, unregister_macro, invoke_macro |
| **Products** | ensure_product, link_project_to_product, unlink_project_from_product, product_inbox, list_products |
| **Setup** | install_precommit_guard, uninstall_precommit_guard |
| **Export** | export_mailbox, add_attachment, get_attachment |
| **Metrics** | list_tool_metrics, list_activity |

---

## 🔧 LAYER 4: LANGUAGE & STACK SPECIFIC

### Package Manager

| Setting | Value |
|---------|-------|
| Package manager | `cargo` (ONLY — never anything else) |
| Rust edition | 2024 |
| Version | Latest stable |
| Dependencies | `Cargo.toml` only |
| Node.js (if needed) | `bun` (not npm/yarn) |

### Build Commands

```bash
# Development build
cargo build --workspace

# Production build
cargo build --workspace --release

# Server-optimized release (uses release-server profile)
cargo build --workspace --profile release-server

# Run development server
cargo run -p mcp-server

# Run production server
cargo run -p mcp-agent-mail --release -- serve
```

### Test Commands

```bash
# Integration tests (MUST use --test-threads=1 for DB isolation)
cargo test -p lib-core --test integration -- --test-threads=1

# Specific BMC tests
cargo test -p lib-core message_bmc

# MCP integration tests
cargo test -p lib-server mcp_integration

# E2E tests
cargo test -p e2e

# All workspace tests
cargo test --workspace
```

### Lint Commands

```bash
# Compiler warnings (fast check)
cargo check --all-targets

# Clippy lints (MUST pass with zero warnings)
cargo clippy --all-targets -- -D warnings

# Format check
cargo fmt --check

# Format fix
cargo fmt
```

### Pre-Commit Checklist

```bash
# Run ALL of these after code changes:
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test -p lib-core --test integration -- --test-threads=1
```

### Code Style Guidelines

#### Backend Model Controller (BMC) Pattern

All business logic in `lib-core` follows the stateless BMC pattern:

```rust
// Stateless controller
pub struct MessageBmc;

impl MessageBmc {
    // All methods take ModelManager as first param
    pub async fn send(
        mm: &ModelManager,
        data: MessageForCreate,
    ) -> Result<Message> {
        // 1. Validate via other BMCs
        // 2. Execute business logic
        // 3. Store via mm.db()
    }
}
```

**Conventions:**
- Controllers: `pub struct FooBmc;` (stateless, no fields)
- Methods: `async fn foo(mm: &ModelManager, ...) -> Result<T>`
- Entity types: `Foo`, `FooForCreate`, `FooForUpdate`

#### Error Handling

```rust
// ✅ GOOD: Proper Result propagation
pub async fn get(mm: &ModelManager, id: i64) -> Result<Agent> {
    let row = stmt.query((id,)).await?
        .next().await?
        .ok_or(Error::AgentNotFound { id })?;
    Ok(Agent::from_row(&row)?)
}

// ❌ BAD: Never unwrap() in src/
let agent = get_agent().unwrap();  // NEVER!

// ✅ ACCEPTABLE: expect() for infallible cases with clear message
let rate = NonZeroU32::new(100).expect("100 is non-zero");
```

#### Strong Types (Newtypes)

```rust
// ✅ GOOD: Domain types prevent argument swapping
pub struct ProjectSlug(String);
pub struct AgentName(String);

fn send(project: ProjectSlug, agent: AgentName)

// ❌ BAD: Primitive obsession
fn send(project: String, agent: String)  // Easy to swap args
```

### Framework-Specific Notes

#### Axum 0.8 (HTTP Layer)

- Use `State<AppState>` for shared state
- Handlers in `lib-server/api/`
- All routes mirror MCP tools

#### rmcp (MCP Protocol)

- Tools defined with `#[tool(description = "...")]` macro
- Params use `#[derive(JsonSchema, Deserialize)]`
- Register in `tool_router!` macro

#### libsql (Database)

- Async SQLite via libsql crate
- Migrations in `migrations/` (auto-run on start)
- Use parameterized queries (SQL injection prevention)

### Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| MCP Throughput | >10k req/s | 15,200 req/s |
| MCP P99 Latency | <10ms | 7.2ms |
| REST /health | >50k req/s | 62,316 req/s |
| Concurrent Agents | 100+ | Verified |

---

## 📚 APPENDIX: TOOL INSTALLATION

### Required Tools

| Tool | Installation | Verify |
|------|--------------|--------|
| cm | See cass-rs repo | `cm --version` |
| cass | `curl \| bash` installer | `cass --version` |
| bd | `brew install bd` | `bd version` |
| bv | Bundled with bd | `bv --version` |
| ubs | Project-specific | `ubs --version` |
| ast-grep | `cargo install ast-grep` | `ast-grep --version` |
| ripgrep | `brew install ripgrep` | `rg --version` |

### Tool Documentation

| Tool | Documentation Location |
|------|----------------------|
| cm | `cm --help`, `cm <cmd> --help` |
| cass | `cass robot-docs guide`, `cass capabilities --json` |
| bd | `bd --help`, `bd <cmd> --help` |
| bv | `bv --robot-help` |

### CLI Tool Preferences

| Use | Not | Example |
|-----|-----|---------|
| `eza` | ls | `eza -la --icons --git` |
| `bat` | cat | `bat file.rs` |
| `rg` | grep | `rg "pattern" --type rust` |
| `fd` | find | `fd -e rs` |

---

## 🆘 TROUBLESHOOTING

### Common Issues

| Problem | Solution |
|---------|----------|
| `cass` launches TUI | Always use `--robot` or `--json` flag |
| `bd` shows "database not found" | Run `bd init --quiet` |
| `cm context` returns nothing | Run `cm init` to initialize playbook |
| Git push fails | Pull with rebase, resolve conflicts, push again |
| Issues not syncing | Run `bd sync` and `bd hooks install` |
| Test DB conflicts | Use `--test-threads=1` for integration tests |

### Health Checks

```bash
# Memory system
cm doctor

# Session search
cass health --json

# Issue tracker
bd ready --json

# Git state
git status

# Rust build
cargo check --all-targets
```

---

## 📝 DOCUMENT MAINTENANCE

This document should be updated when:
- New tools are added to the project
- Workflow changes are made
- Common issues are discovered
- Project-specific sections need updating

**Version**: 1.0.0
**Last Updated**: 2025-12-17
**Maintainer**: Avyukth

---

## 📖 RELATED DOCUMENTATION

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — System architecture
- [docs/WALKTHROUGH.md](docs/WALKTHROUGH.md) — Usage walkthrough
- [.env.example](.env.example) — All environment variables
- [scripts/integrations/](scripts/integrations/) — Agent integration configs
- [MCP Protocol](https://modelcontextprotocol.io)

---

*Remember: When in doubt, run `cm context "<your task>"` and `bd ready` to get oriented.*
