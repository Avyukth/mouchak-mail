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

### Step-by-Step Agent Workflow

```
┌─────────────────────────────────────────────────────────────────┐
│                    AGENT EXECUTION FLOW                          │
└─────────────────────────────────────────────────────────────────┘

 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 1: INITIALIZATION                                       │
 ├──────────────────────────────────────────────────────────────┤
 │  1. cm context "<task>"     # Get procedural memory rules    │
 │  2. bd ready --json         # Find ready work (no blockers)  │
 │  3. bd show <id> --json     # Read issue details             │
 │  4. bd update <id> --status in_progress                      │
 └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 2: SANDBOX CREATION                                     │
 ├──────────────────────────────────────────────────────────────┤
 │  5. git worktree add .sandboxes/agent-<id> feature/<id>      │
 │  6. cd .sandboxes/agent-<id>                                 │
 │     └── Agent now has isolated workspace                     │
 └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 3: EXECUTION                                            │
 ├──────────────────────────────────────────────────────────────┤
 │  7. Read code, understand requirements                        │
 │  8. Make changes (edit, create, delete files)                │
 │  9. Run tests: cargo test                                    │
 │ 10. Run linter: cargo clippy --workspace                     │
 │                                                               │
 │  If discovered issues:                                        │
 │  bd create "Bug: X" -t bug --deps discovered-from:<id>       │
 └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 4: QUALITY GATES                                        │
 ├──────────────────────────────────────────────────────────────┤
 │ 11. cargo test --workspace        # All tests pass?          │
 │ 12. cargo clippy -- -D warnings   # No warnings?             │
 │ 13. cargo fmt --check             # Formatted?               │
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
 │ 14. git add -A                                                │
 │ 15. git commit -m "feat: <description> (closes bd-<id>)"     │
 │ 16. cd ../..                      # Back to main repo        │
 │ 17. git checkout main                                        │
 │ 18. git merge feature/<id>                                   │
 │ 19. git worktree remove .sandboxes/agent-<id>                │
 │ 20. git branch -d feature/<id>                               │
 └──────────────────────────────────────────────────────────────┘
                              │
                              ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ PHASE 6: CLEANUP                                              │
 ├──────────────────────────────────────────────────────────────┤
 │ 21. bd close <id> --reason "Completed"                       │
 │ 22. bd sync                       # Sync with git            │
 │ 23. git add .beads/issues.jsonl                              │
 │ 24. git commit -m "chore: close bd-<id>"                     │
 └──────────────────────────────────────────────────────────────┘
```

### Detailed Step-by-Step Commands

**Step 1: Get Procedural Memory Context**
```bash
cm context "implement user authentication"
# Returns relevant rules from ~/.cass-memory/playbook.yaml
# Example output:
# - [b-123] Always use proper error handling with Result types
# - [b-456] Run cargo clippy before committing
```

**Step 2: Find Ready Work**
```bash
bd ready --json
# Returns issues with no blockers, sorted by priority
# Example output:
# [{"id":"bd-abc","title":"Add login endpoint","priority":1,"status":"open"}]
```

**Step 3: Read Issue Details**
```bash
bd show bd-abc --json
# Returns full issue: description, acceptance criteria, dependencies
```

**Step 4: Claim the Issue**
```bash
bd update bd-abc --status in_progress --json
# Atomically claims issue - prevents other agents from taking it
```

**Step 5-6: Create Sandbox**
```bash
git worktree add .sandboxes/agent-bd-abc feature/bd-abc
cd .sandboxes/agent-bd-abc
# Now working in isolated environment
```

**Step 7-10: Execute Work**
```bash
# Read, understand, implement
# Make changes to files
# Run tests frequently
cargo test -p lib-core
```

**Step 11-13: Quality Gates**
```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
# All must pass before merge
```

**Step 14-20: Commit and Merge**
```bash
git add -A
git commit -m "feat(auth): add login endpoint (closes bd-abc)"
cd ../..
git checkout main
git merge feature/bd-abc
git worktree remove .sandboxes/agent-bd-abc
git branch -d feature/bd-abc
```

**Step 21-24: Close and Sync**
```bash
bd close bd-abc --reason "Implemented login endpoint with tests"
bd sync
git add .beads/issues.jsonl
git commit -m "chore: close bd-abc"
```

### Failure Handling

**If quality gates fail:**
```bash
# Stay in sandbox, file blocker issue
bd create "Quality gate failed: clippy warnings in auth module" \
  -t bug -p 0 \
  --deps discovered-from:bd-abc

# Release the issue (reopen for retry)
bd update bd-abc --status open --notes "Blocked by quality gates"

# Optionally keep sandbox for debugging
# Or remove it:
cd ../..
git worktree remove --force .sandboxes/agent-bd-abc
git branch -D feature/bd-abc
```

**If blocked by external dependency:**
```bash
# File blocker issue
bd create "Blocked: need API spec from team" \
  -t task -p 1 \
  --label no-auto-claim \
  --deps blocks:bd-abc

# Update original issue
bd update bd-abc --status blocked --notes "Waiting on bd-xyz"
```

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
