# CLAUDE.md - Claude Code Instructions

> See [AGENTS.md](./AGENTS.md) for full project documentation.
> This file adds Claude-specific reasoning guidelines and environment context.

## Quick Start

```bash
# 1. Get procedural memory context
cm context "<your task>"

# 2. Check what needs to be done
bd ready --json

# 3. Run development servers
make dev

# 4. Run tests
make test
```

---

## Multi-Agent Workflow

This project supports concurrent multi-agent development using git worktrees for isolation.

### Git Worktree Isolation

Each agent works in an isolated git worktree to prevent conflicts:

```bash
# Create worktree for agent task
git worktree add .sandboxes/agent-bd-abc feature/bd-abc

# Agent works in sandbox
cd .sandboxes/agent-bd-abc
# ... make changes ...

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

**Directory Structure:**
```
.sandboxes/
├── agent-bd-abc/     # Agent 1 working on bd-abc
├── agent-bd-xyz/     # Agent 2 working on bd-xyz
└── agent-bd-123/     # Agent 3 working on bd-123
```

### Hash-Based Issue IDs (Collision-Free)

Beads uses hash-based IDs enabling collision-free multi-agent workflows:

```bash
# Agent A creates: bd-a1b2 (on branch feature-auth)
# Agent B creates: bd-f14c (on branch feature-payments)
# Git merge succeeds cleanly - no collision!
```

---

## Multi-Agent Memory System

This project uses two complementary memory systems for cross-agent learning:

### cass-rs (`cm`) - Procedural Memory

**Before starting any non-trivial task:**

```bash
cm context "<task description>"
```

This returns:
- **Relevant rules** from the playbook (scored by task relevance)
- **Anti-patterns** to avoid (things that caused problems)
- **Historical context** from past sessions

**Agent Protocol:**

| Phase | Action |
|-------|--------|
| **START** | Run `cm context "<task>"` before non-trivial work |
| **WORK** | Reference rule IDs: "Following b-8f3a2c, checking token expiry..." |
| **FEEDBACK** | Leave inline comments: `// [cass: helpful b-xyz] - reason` |
| **END** | Just finish. Learning happens automatically via reflection. |

**Inline Feedback (Optional):**

```rust
// [cass: helpful b-8f3a2c] - this rule saved debugging time
// [cass: harmful b-x7k9p1] - this advice was wrong for our use case
```

### beads (`bd`) - Task Memory

**Distributed issue tracker via git:**

```bash
bd ready --json           # Find ready work (no blockers)
bd update <id> --status in_progress --json
bd create "Bug: X" -t bug -p 0 --json
bd close <id> --reason "Done" --json
```

**Hash-based IDs** enable collision-free multi-agent workflows:
- Agent A creates `bd-a1b2` on branch `feature-auth`
- Agent B creates `bd-f14c` on branch `feature-payments`
- Git merge succeeds cleanly

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
1. Look beyond obvious causes — the root issue may be deeper
2. Prioritize hypotheses by likelihood, but don't discard low-probability ones
3. Each hypothesis may need multiple steps to verify

### 4. Adaptability

After each observation:
- Does the result change the plan?
- If hypotheses disproven → generate new ones from gathered info

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

**Rust** → cargo (standard)
```bash
cargo build
cargo test
cargo clippy
```

**JavaScript** → bun (not npm/yarn)
```bash
bun install
bun add pkg
bun run dev
```

**Python** → uv (not pip/venv)
```bash
uv add pkg
uv run python x.py
uvx ruff check .
```

---

## Project Workflow

### Starting Work

```bash
# 1. Get procedural memory
cm context "<task description>"

# 2. Find ready work
bd ready --json

# 3. Claim task
bd update <id> --status in_progress --json

# 4. Read context
bd show <id> --json
```

### During Work

```bash
# Build and test
make build
make test

# Found something new?
bd create "Bug: X" -t bug --deps discovered-from:<current-id> --json

# Rule helped/hurt?
# Leave inline comment: // [cass: helpful b-xyz] - reason
```

### Completing Work

```bash
# 1. Verify quality gates
cargo clippy --workspace
cargo test -p lib-core --test integration

# 2. Close the task
bd close <id> --reason "Implemented X" --json

# 3. Sync beads
bd sync

# 4. Commit with beads
git add -A && git commit -m "feat: X (closes bd-Y)"
```

---

## Tech Stack Reference

| Layer | Technology | Key Files |
|-------|------------|-----------|
| Core | Rust, lib-core | `crates/libs/lib-core/` |
| API | Axum 0.8 | `crates/services/mcp-server/` |
| MCP | rmcp | `crates/services/mcp-stdio/` |
| Frontend | SvelteKit → Leptos | `crates/services/web-ui/` |
| Database | libsql (SQLite) | `migrations/*.sql` |
| Tracker | beads (`bd`) | `.beads/issues.jsonl` |
| Memory | cass-rs (`cm`) | `~/.cass-memory/playbook.yaml` (global) |

---

## Critical Rules

**DO:**
- Run `cm context "<task>"` before non-trivial work
- Use `bd` for ALL task tracking
- Run `make test` before completing work
- Use `--json` flag for programmatic parsing
- Commit `.beads/issues.jsonl` with code
- Leave `// [cass: helpful/harmful b-xyz]` feedback

**DON'T:**
- Create markdown TODO lists
- Use `unwrap()` in production code
- Skip claiming issues before working
- Ignore clippy warnings
- Forget to sync beads before session end

---

## Hooks (Automatic)

These run automatically via `.claude/settings.local.json`:

| Hook | Trigger | Action |
|------|---------|--------|
| SessionStart | Session begins | `bd ready --json` |
| UserPromptSubmit | Each prompt | `cm context "<prompt>"` |
| PreCompact | Before compaction | `bd ready --json` |

---

## Quick Reference

```bash
# Memory Systems
cm context "<task>"      # Get procedural memory
cm stats                 # Playbook health
cm mark <id> helpful     # Provide feedback

# Task Tracking
bd ready --json          # What to work on
bd update <id> --status in_progress
bd close <id> --reason "Done"
bd sync                  # Sync with git

# Development
make dev                 # Run API + Web UI
make test                # Run all tests

# Quality
cargo clippy --workspace
cargo fmt --all
pmat rust-project-score  # Quality metrics
```
