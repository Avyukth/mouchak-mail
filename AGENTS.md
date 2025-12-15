# AGENTS.md - AI Coding Agent Instructions

> Universal instructions for AI coding agents (Claude, Gemini, Codex, GPT, Copilot, etc.) working on this project.

## Quick Start

```bash
# 1. Find ready work
bd ready --json

# 2. Claim a task
bd update <id> --status in_progress --json

# 3. Run development servers
make dev

# 4. Run tests
make test
```

---

## Project Overview

**Goal**: "Gmail for coding agents" - A multi-agent messaging system implemented in Rust with SvelteKit frontend.

**Tech Stack**:

| Layer | Technology | Key Files |
|-------|------------|-----------|
| Core | Rust, lib-core | `crates/libs/lib-core/` |
| API | Axum 0.8 | `crates/services/mcp-server/` |
| MCP | rmcp | `crates/services/mcp-stdio/` |
| Frontend | SvelteKit | `crates/services/web-ui/` |
| Database | libsql (SQLite) | `migrations/*.sql` |
| Tracker | beads (`bd`) | `.beads/issues.jsonl` |

**Key Directories**:

```
mcp-agent-mail-rs/
├── crates/
│   ├── libs/lib-core/      # Domain logic, BMC pattern
│   └── services/
│       ├── mcp-server/     # Axum REST API
│       ├── mcp-stdio/      # MCP protocol server
│       ├── mcp-cli/        # CLI for testing
│       └── web-ui/         # SvelteKit frontend
├── migrations/             # SQL schema
├── docs/                   # Documentation
├── .beads/                 # Issue tracker
└── scripts/                # Utility scripts
```

---

## Issue Tracking with bd (beads)

**CRITICAL**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs or other tracking methods.

> **Warning:** Do not edit `.beads/*.jsonl` directly; only use `bd` commands.

### Quick Reference

```bash
# Find ready work (no blockers)
bd ready --json

# Create new issues
bd create "Issue title" -t bug|feature|task -p 0-4 --json

# Claim and update
bd update <id> --status in_progress --json

# Complete work
bd close <id> --reason "Completed" --json

# View all issues
bd list --json
```

### Issue Types & Priorities

| Type | Use For |
|------|---------|
| `epic` | Large features with subtasks |
| `feature` | New functionality |
| `task` | Work items |
| `bug` | Something broken |
| `chore` | Maintenance |

| Priority | Meaning |
|----------|---------|
| 0 | Critical (security, data loss) |
| 1 | High (major features) |
| 2 | Medium (default) |
| 3 | Low (polish) |
| 4 | Backlog |

---

## Multi-Agent Workflow

This project supports concurrent multi-agent development using git worktrees for isolation.

### Agent Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    AGENT EXECUTION FLOW                          │
└─────────────────────────────────────────────────────────────────┘

 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 1: INITIALIZATION                                       │
 ├──────────────────────────────────────────────────────────────┤
 │  1. bd ready --json         # Find ready work (no blockers)  │
 │  2. bd show <id> --json     # Read issue details             │
 │  3. bd update <id> --status in_progress                      │
 └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 2: SANDBOX CREATION                                     │
 ├──────────────────────────────────────────────────────────────┤
 │  4. git worktree add .sandboxes/agent-<id> feature/<id>      │
 │  5. cd .sandboxes/agent-<id>                                 │
 │     └── Agent now has isolated workspace                     │
 └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 3: EXECUTION                                            │
 ├──────────────────────────────────────────────────────────────┤
 │  6. Read code, understand requirements                        │
 │  7. Make changes (edit, create, delete files)                │
 │  8. Run tests: cargo test                                    │
 │  9. Run linter: cargo clippy --workspace                     │
 │                                                               │
 │  If discovered issues:                                        │
 │  bd create "Bug: X" -t bug --deps discovered-from:<id>       │
 └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 4: QUALITY GATES                                        │
 ├──────────────────────────────────────────────────────────────┤
 │ 10. cargo test --workspace        # All tests pass?          │
 │ 11. cargo clippy -- -D warnings   # No warnings?             │
 │ 12. cargo fmt --check             # Formatted?               │
 │                                                               │
 │  ┌─────────┐                      ┌─────────┐                │
 │  │  PASS   │                      │  FAIL   │                │
 │  └────┬────┘                      └────┬────┘                │
 │       │                                │                      │
 │       ▼                                ▼                      │
 │  Continue to                      File blocker issue         │
 │  Phase 5                          bd create "Gate failed"    │
 │                                   -t bug -p 0                 │
 └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 5: COMMIT & MERGE (on success)                          │
 ├──────────────────────────────────────────────────────────────┤
 │ 13. git add -A                                                │
 │ 14. git commit -m "feat: <description> (closes bd-<id>)"     │
 │ 15. cd ../..                      # Back to main repo        │
 │ 16. git checkout main                                        │
 │ 17. git merge feature/<id>                                   │
 │ 18. git worktree remove .sandboxes/agent-<id>                │
 │ 19. git branch -d feature/<id>                               │
 └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 6: CLEANUP                                              │
 ├──────────────────────────────────────────────────────────────┤
 │ 20. bd close <id> --reason "Completed"                       │
 │ 21. git add .beads/issues.jsonl                              │
 │ 22. git commit -m "chore: close bd-<id>"                     │
 └──────────────────────────────────────────────────────────────┘
```

### Git Worktree Isolation

Each agent works in an isolated git worktree to prevent conflicts:

```bash
# Create worktree for agent task
git worktree add .sandboxes/agent-bd-abc feature/bd-abc
cd .sandboxes/agent-bd-abc

# On success: merge changes
git checkout main
git merge feature/bd-abc
git worktree remove .sandboxes/agent-bd-abc

# On failure: discard sandbox
git worktree remove --force .sandboxes/agent-bd-abc
git branch -D feature/bd-abc
```

**Benefits:**
- Main repo stays clean
- Multiple agents work in parallel without conflicts
- Easy rollback on failure
- Each agent has isolated file system

### Multi-Agent Handoff Protocol

When handing off work between agents:

1. **Outgoing agent**: Update issue with progress note
   ```bash
   bd comment <id> "Progress: completed A and B, remaining: C and D"
   bd update <id> --status open --json  # Release claim
   ```

2. **Incoming agent**: Check recent activity
   ```bash
   bd show <id> --json           # See full issue with comments
   bd ready --json               # See unblocked work
   ```

3. **Coordination via git**:
   ```bash
   git pull                      # Get latest .beads/issues.jsonl
   bd ready --json               # Beads auto-imports from JSONL
   ```

---

## Reasoning & Planning Framework

Before taking any action, reason through these steps:

### 1. Logical Dependencies & Constraints

Analyze the action against these factors (resolve conflicts in order):

1. **Policy rules**: Check AGENTS.md constraints (use `bd`, quality gates, etc.)
2. **Order of operations**: Will this action prevent a later necessary action?
3. **Prerequisites**: What information/actions are needed first?
4. **User preferences**: Explicit constraints from the conversation

### 2. Risk Assessment

| Risk Level | When to Proceed Without Asking |
|------------|--------------------------------|
| **LOW** | Missing optional params, exploratory searches |
| **MEDIUM** | Modifying existing code, adding deps |
| **HIGH** | Deleting files, changing configs, external calls |

**Prefer action over asking** unless HIGH risk or Rule 1 issues.

### 3. Abductive Reasoning

When debugging:
1. Look beyond obvious causes - the root issue may be deeper
2. Prioritize hypotheses by likelihood, but don't discard low-probability ones
3. Each hypothesis may need multiple steps to verify

### 4. Adaptability

After each observation:
- Does the result change the plan?
- If hypotheses disproven -> generate new ones from gathered info

### 5. Persistence

- On **transient** errors ("please try again"): RETRY
- On **other** errors: Change strategy, don't repeat same call
- Don't give up until reasoning is exhausted

---

## Environment Context

### CLI Tools (prefer these)

| Use | Not | Example |
|-----|-----|---------|
| `eza` | ls | `eza -la --icons --git` |
| `bat` | cat | `bat file.rs` |
| `rg` | grep | `rg "pattern" --type rust` |
| `fd` | find | `fd -e rs` |

### Package Managers

**Rust** -> cargo (standard)
```bash
cargo build
cargo test
cargo clippy
```

**JavaScript** -> bun (not npm/yarn)
```bash
bun install
bun add pkg
bun run dev
```

**Python** -> uv (not pip/venv)
```bash
uv add pkg
uv run python x.py
uvx ruff check .
```

---

## Quality Gates

Before marking work complete:

- [ ] Code compiles: `cargo build`
- [ ] No warnings: `cargo clippy -- -D warnings`
- [ ] Tests pass: `cargo test`
- [ ] Formatted: `cargo fmt --check`
- [ ] No `unwrap()` in production code
- [ ] Update docs if behavior changed

---

## Critical Rules

**DO:**
- Use `bd` for ALL task tracking
- Use `--json` flag for programmatic parsing
- Link discovered work with `discovered-from` dependencies
- Check `bd ready` before asking "what should I work on?"
- Commit `.beads/issues.jsonl` with code changes

**DON'T:**
- Create markdown TODO lists
- Edit `.beads/*.jsonl` directly (only use `bd` commands)
- Use `unwrap()` in production code
- Skip claiming issues before working
- Ignore clippy warnings

---

## Quick Reference

```bash
# Task Tracking
bd ready --json          # What to work on
bd update <id> --status in_progress
bd close <id> --reason "Done"

# Development
make dev                 # Run API + Web UI
make test                # Run all tests
make build               # Build all

# Quality
cargo clippy --workspace
cargo fmt --all
cargo test --workspace

# Git Worktrees
git worktree add .sandboxes/agent-<id> feature/<id>
git worktree remove .sandboxes/agent-<id>
```

---

## References

- [Project Plan](docs/PROJECT_PLAN.md)
- [Python Source](https://github.com/Dicklesworthstone/mcp_agent_mail)
- [Beads Issue Tracker](https://github.com/steveyegge/beads)
- [MCP Protocol](https://modelcontextprotocol.io)
