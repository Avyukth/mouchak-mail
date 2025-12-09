# CLAUDE.md - Claude Code Instructions

> See [AGENTS.md](./AGENTS.md) for full project documentation.
> This file adds Claude-specific reasoning guidelines and environment context.

## Quick Start

```bash
# Check what needs to be done
bd ready --json

# Run development servers
make dev

# Run tests
make test
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
# 1. Orient
bd ready --json          # What's available?
bat AGENTS.md            # Refresh on rules

# 2. Claim
bd update <id> --status in_progress --json

# 3. Read context
bd show <epic-id> --json # Parent epic details
```

### During Work

```bash
# Build and test
make build
make test

# Found something new?
bd create "Bug: X" -t bug --deps discovered-from:<current-id> --json
```

### Completing Work

```bash
# 1. Verify quality gates
cargo clippy --workspace
cargo test -p lib-core --test integration

# 2. Close the task
bd close <id> --reason "Implemented X" --json

# 3. Commit with beads
git add -A && git commit -m "feat: X (closes bd-Y)"
```

---

## Tech Stack Reference

| Layer | Technology | Key Files |
|-------|------------|-----------|
| Core | Rust, lib-core | `crates/libs/lib-core/` |
| API | Axum 0.8 | `crates/services/mcp-server/` |
| MCP | rmcp | `crates/services/mcp-stdio/` |
| Frontend | SvelteKit, Bun | `crates/services/web-ui/` |
| Database | libsql (SQLite) | `migrations/*.sql` |
| Tracker | beads (`bd`) | `.beads/issues.jsonl` |

---

## Critical Rules

✅ **DO**:
- Use `bd` for ALL task tracking
- Run `make test` before completing work
- Use `--json` flag for programmatic parsing
- Commit `.beads/issues.jsonl` with code

❌ **DON'T**:
- Create markdown TODO lists
- Use `unwrap()` in production code
- Skip claiming issues before working
- Ignore clippy warnings

---

## Quick Reference

```bash
# Development
make dev                 # Run API + Web UI
make test                # Run all tests

# Beads
bd ready --json          # What to work on
bd update <id> --status in_progress
bd close <id> --reason "Done"

# Quality
cargo clippy --workspace
cargo fmt --all
```
