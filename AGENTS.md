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
| `git push` | **NEVER**|
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
| **AI-supervised execution** | vc | `vc run` / `vc status` |
| **Quality gates / TDG** | pmat | `pmat analyze tdg` / `pmat mutate` |
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

### 🤖 vc (AI-Supervised Executor) — Autonomous Agent Colony

**What it does**: Orchestrates AI coding agents with supervision, quality gates, and automatic work discovery. VC claims issues from bd, spawns coding agents, and creates follow-on issues for discovered work.

#### Core Concepts

| Concept | Description |
|---------|-------------|
| **Zero Framework Cognition (ZFC)** | All decisions delegated to AI—no heuristics or regex in orchestration |
| **Issue-Oriented Orchestration** | Work flows through bd issue tracker with explicit dependencies |
| **Nondeterministic Idempotence** | Operations can crash and resume; AI figures out where to continue |
| **AI Supervision** | Assessment before work, analysis after—catches mistakes early |

#### Workflow

```
1. User: "Fix bug X" (or vc finds ready work)
2. VC claims issue atomically from bd
3. AI assesses: strategy, steps, risks
4. Agent executes the work (Amp/Claude Code)
5. AI analyzes: completion, punted items, discovered bugs
6. Auto-create follow-on issues
7. Quality gates enforce standards
8. Repeat until done
```

#### Key Commands

| Command | Purpose |
|---------|---------|
| `vc run` | Start executor loop |
| `vc status` | View executor state |
| `vc activity` | View activity feed |
| `vc repl` | Interactive natural language interface |

#### Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `ANTHROPIC_API_KEY` | Yes | AI supervision (assessment/analysis) |
| `VC_DEBUG_PROMPTS` | No | Log full prompts sent to agents |
| `VC_DEBUG_EVENTS` | No | Log JSON event parsing |
| `VC_DEBUG_STATUS` | No | Log issue status changes |
| `VC_DEBUG_WORK_SELECTION` | No | Log work selection filtering |

#### Blocker-First Prioritization

VC uses blocker-first prioritization to ensure missions complete:

1. Baseline-failure issues (self-healing mode)
2. Discovered blockers (`discovered:blocker` label)
3. Regular ready work (sorted by priority)
4. Discovered related work (`discovered:related` label)

**Work starvation is acceptable** for mission convergence—blockers always take precedence.

#### Executor Exclusion

Use `no-auto-claim` label **ONLY** for:
- External coordination (other teams, approvals)
- Human creativity (UX, branding, marketing)
- Business judgment (pricing, legal, compliance)
- Pure research (no clear deliverable)

**Everything else is fair game**—VC has robust safety nets (quality gates, AI supervision, sandbox isolation, self-healing).

```bash
# Prevent executor from auto-claiming an issue
bd label add <id> no-auto-claim
```

#### Safety Nets

VC has robust safety mechanisms that allow it to tackle hard problems:

| Safety Net | Purpose |
|------------|---------|
| **Quality gates** | Test/lint/build validate changes before merge |
| **AI supervision** | Assessment + analysis guides approach, catches mistakes |
| **Sandbox isolation** | Git worktrees prevent contamination of main branch |
| **Self-healing** | Automatically fixes broken baselines |
| **Activity feed** | Full visibility into what's happening |
| **Human intervention** | Possible at any time via CLI |

#### Integration with bd

VC consumes issues from bd and creates follow-on work automatically:

```bash
# VC claims ready work
bd ready --json  # VC queries this

# VC creates discovered issues
bd create "Found during work" --deps discovered-from:<parent> --json

# VC closes completed work
bd close <id> --reason "Completed" --json
```

---

### 📊 pmat (Pragmatic Multi-language Agent Toolkit) — Quality Gates & Analysis

**What it does**: Zero-configuration code quality analysis, technical debt grading, mutation testing, and AI context generation. Supports 17+ languages.

#### Core Capabilities

| Feature | Purpose |
|---------|---------|
| **Context Generation** | Deep analysis for Claude, GPT, and other LLMs |
| **Technical Debt Grading (TDG)** | A+ through F scoring with 6 orthogonal metrics |
| **Mutation Testing** | Test suite quality validation (85%+ kill rate target) |
| **Repository Scoring** | Quantitative health assessment (0-211 scale) |
| **Semantic Search** | Natural language code discovery |
| **Quality Gates** | Pre-commit hooks, CI/CD integration |

#### Essential Commands

```bash
# Generate AI-ready context
pmat context --output context.md --format llm-optimized

# Analyze code complexity
pmat analyze complexity

# Grade technical debt (A+ through F)
pmat analyze tdg

# Score repository health (Rust projects)
pmat rust-project-score           # Fast mode (~3 min)
pmat rust-project-score --full    # Comprehensive (~10-15 min)

# Run mutation testing
pmat mutate --target src/ --threshold 85
```

#### Technical Debt Grading (TDG)

TDG provides six orthogonal metrics for accurate quality assessment:

```bash
pmat analyze tdg                       # Project-wide grade
pmat analyze tdg --include-components  # Per-component breakdown
pmat tdg baseline create               # Create quality baseline
pmat tdg check-regression              # Detect quality degradation
```

**Grading Scale**:

| Grade | Meaning |
|-------|---------|
| A+/A | Excellent quality, minimal debt |
| B+/B | Good quality, manageable debt |
| C+/C | Needs improvement |
| D/F | Significant technical debt |

#### Quality Baseline Workflow

```bash
# 1. Create baseline (do this when quality is good)
pmat tdg baseline create --output .pmat/baseline.json

# 2. Check for regressions (in CI or pre-commit)
pmat tdg check-regression \
  --baseline .pmat/baseline.json \
  --max-score-drop 5.0 \
  --fail-on-regression
```

#### Mutation Testing

Validates test suite effectiveness by introducing mutations and checking if tests catch them:

```bash
pmat mutate --target src/lib.rs           # Single file
pmat mutate --target src/ --threshold 85  # Quality gate (85% kill rate)
pmat mutate --failures-only               # CI optimization (faster)
```

**Supported languages**: Rust, Python, TypeScript, JavaScript, Go, C++

#### Git Hooks Integration

```bash
pmat hooks install                     # Install pre-commit hooks
pmat hooks install --tdg-enforcement   # With TDG quality gates
pmat hooks status                      # Check hook status
```

#### Workflow Prompts

Pre-configured AI prompts enforcing EXTREME TDD:

```bash
pmat prompt --list                     # Available prompts
pmat prompt code-coverage              # 85%+ coverage enforcement
pmat prompt debug                      # Five Whys analysis
pmat prompt quality-enforcement        # All quality gates
```

#### MCP Server Mode

```bash
# Start MCP server for Claude Code, Cline, etc.
pmat mcp
```

Provides 19 tools for AI agents via MCP protocol.

#### CI/CD Integration Example

```yaml
# .github/workflows/quality.yml
- run: pmat analyze tdg --fail-on-violation --min-grade B
- run: pmat mutate --target src/ --threshold 80
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

**What it does**: Production-grade async coordination for multi-agent workflows via messaging, file reservations, and build slot management. Provides "Gmail for coding agents" — 45 MCP tools for agent coordination.

**Server**: `http://localhost:8765` (start with `make dev-api`)

#### When to Use

- ✅ Multiple agents working concurrently on same repo
- ✅ Need to prevent file conflicts (file reservations)
- ✅ Real-time coordination required (messaging + threads)
- ✅ CI/CD isolation (build slots)
- ✅ Cross-repo coordination (products)
- ✅ Single agent workflows (works the same)

#### MCP Tools Reference (45 total)

| Category | Tools | Description |
|----------|-------|-------------|
| **Infrastructure** | `ensure_project`, `list_projects`, `get_project_info` | Project lifecycle |
| **Agent** | `register_agent`, `list_agents`, `get_agent_profile` | Agent identity |
| **Messaging** | `send_message`, `reply_message`, `check_inbox`, `list_outbox`, `get_message`, `search_messages` | Core messaging |
| **Read Status** | `mark_message_read`, `acknowledge_message` | Message acknowledgment |
| **Threads** | `list_threads`, `summarize_thread`, `summarize_threads` | Conversation tracking |
| **Contacts** | `request_contact`, `respond_contact`, `list_contacts`, `set_contact_policy` | Agent routing |
| **File Reservations** | `reserve_file`, `release_reservation`, `list_file_reservations`, `force_release_reservation`, `renew_file_reservation`, `file_reservation_paths` | Conflict prevention |
| **Build Slots** | `acquire_build_slot`, `release_build_slot`, `renew_build_slot` | CI/CD isolation |
| **Macros** | `list_macros`, `register_macro`, `invoke_macro` | Automation |
| **Products** | `ensure_product`, `link_project_to_product`, `list_products`, `product_inbox` | Cross-repo coordination |
| **Export** | `export_mailbox` | Archive to HTML/JSON/Markdown |
| **Attachments** | `add_attachment`, `get_attachment` | File sharing |
| **Pre-commit** | `install_precommit_guard`, `uninstall_precommit_guard` | Conflict detection |
| **Metrics** | `list_tool_metrics`, `get_tool_stats`, `list_activity` | Usage analytics |

#### Setup Workflow

```bash
# 1. Start server (if not running)
make dev-api  # Starts on :8765

# 2. Register identity (MCP call)
ensure_project(project_key="/path/to/repo", human_key="my-project")
register_agent(project_key="/path/to/repo", agent_name="claude-1", program="claude-code")

# 3. Reserve files before editing (prevents conflicts)
file_reservation_paths(
  project_key="/path/to/repo",
  agent_name="claude-1",
  paths=["src/**/*.rs", "Cargo.toml"],
  ttl_seconds=3600,
  exclusive=true
)

# 4. Communicate via threads
send_message(
  project_key="/path/to/repo",
  sender_name="claude-1",
  recipient_names=["claude-2"],
  subject="Found issue in auth module",
  body_md="See src/auth.rs:42...",
  thread_id="FEAT-123"
)

# 5. Check for messages
check_inbox(project_key="/path/to/repo", agent_name="claude-1", unread_only=true)
acknowledge_message(project_key="/path/to/repo", message_id=123, agent_name="claude-1")

# 6. Release reservations when done
release_reservation(project_key="/path/to/repo", reservation_id=1)
```

#### REST API (Alternative to MCP)

```bash
# Health check
curl http://localhost:8765/api/health

# Send message via REST
curl -X POST http://localhost:8765/api/message/send \
  -H "Content-Type: application/json" \
  -d '{"project_key":"/path/to/repo","sender_name":"claude-1","recipient_names":["claude-2"],"subject":"Test","body_md":"Hello"}'

# Check inbox
curl -X POST http://localhost:8765/api/inbox \
  -H "Content-Type: application/json" \
  -d '{"project_key":"/path/to/repo","agent_name":"claude-1"}'
```

#### Resources

- `resource://inbox/{Agent}?project=<path>&limit=20`
- `resource://thread/{id}?project=<path>&include_bodies=true`

**Tip**: Set `AGENT_NAME` env var for pre-commit guard to block conflicting commits.

---

## 🔄 LAYER 2: SESSION WORKFLOW

### Git Worktree Flow (Sandbox Isolation)

**What it does**: Git worktrees allow multiple working directories from the same repository, each checking out a different branch in its own directory. They share repository history and objects but keep work directories isolated. Available in Git 2.5+.

#### When to Use Worktrees

- ✅ Working on risky/experimental changes
- ✅ Handling interruptions (hotfixes) without stashing
- ✅ Parallel feature development
- ✅ Code reviews and PR testing in isolation
- ✅ AI executor sandboxes (VC uses this)
- ✅ Long-running tasks (test suites) while continuing other work
- ✅ Comparing branches side-by-side

#### Core Commands

| Command | Purpose | Key Options |
|---------|---------|-------------|
| `git worktree add <path> [<branch>]` | Create new worktree | `-b <new-branch>`: Create new branch<br>`--detach`: Detach HEAD<br>`--no-checkout`: Skip checkout |
| `git worktree list` | Show all worktrees | `--verbose`: Add state details<br>`--porcelain`: Machine-readable |
| `git worktree remove <path>` | Delete worktree | `-f`: Force (unclean/locked) |
| `git worktree prune` | Clean stale metadata | `--expire <time>`: Age threshold |
| `git worktree lock <path>` | Prevent pruning | `--reason <string>`: Add note |
| `git worktree unlock <path>` | Allow management | |
| `git worktree move <old> <new>` | Relocate worktree | |
| `git worktree repair` | Fix broken links | After manual moves |

#### Basic Workflow

```bash
# 1. Create worktree for existing branch
git worktree add ../project-feature feature-branch

# 2. Create worktree with NEW branch tracking remote
git worktree add -b bugfix-123 ../bugfix-123 origin/main

# 3. Work in the worktree
cd ../bugfix-123
# ... make changes, commit, push ...

# 4. Return and clean up
cd ../project
git worktree remove ../bugfix-123
```

#### Handling Interruptions (Hotfix Pattern)

When urgent work arrives, don't stash—create a worktree:

```bash
# 1. Create hotfix worktree (doesn't affect current work)
git worktree add ../hotfix hotfix-branch

# 2. Fix the issue
cd ../hotfix
# ... fix, test, commit, push, deploy ...

# 3. Clean up
cd ../project
git worktree remove ../hotfix
```

#### Bare Repository Workflow (Advanced)

For cleaner organization, use a bare repo as central hub:

```bash
# 1. Clone as bare repository
git clone --bare <repo-url> project.git

# 2. Create worktrees from bare repo
cd project.git
git worktree add ../main main
git worktree add ../feature-x feature-x
```

**Directory structure**:
```
project/
├── project.git/     # Bare repo (central hub)
├── main/            # Worktree for main branch
├── feature-x/       # Worktree for feature
└── bugfix-123/      # Worktree for bugfix
```

**Bare repo caveats**:
- Fetching remote branches may require manual ref setup
- Some prefer non-bare repos for simpler remote handling
- Use `git config remote.origin.fetch "+refs/heads/*:refs/remotes/origin/*"` to fix fetch issues

#### PR Review Workflow (with GitHub CLI)

```bash
# 1. Create worktree for PR review
git worktree add ../pr-review pr-branch
# Or with GitHub CLI:
cd ../pr-review && gh pr checkout <PR_NUMBER>

# 2. Test and review
# ... run tests, review code ...

# 3. Clean up
git worktree remove ../pr-review
```

#### Worktree + bd Integration

**IMPORTANT**: bd daemon mode is NOT supported in worktrees. Use `--no-daemon` flag:

```bash
# In a worktree, always use --no-daemon
bd --no-daemon ready
bd --no-daemon update <id> --status in_progress
bd --no-daemon close <id> --reason "Done"
```

#### Best Practices

| Practice | Reason |
|----------|--------|
| **Limit to 3-5 active worktrees** | Each duplicates files; manage disk space |
| **Use descriptive naming** | `project-feature-auth` not `wt1` |
| **Place as siblings** | `../project-feature` not `./worktrees/feature` |
| **Remove promptly when done** | Avoid accumulating stale worktrees |
| **Commit frequently** | Sync with main repo to avoid conflicts |
| **Run `git worktree prune` regularly** | Clean up stale metadata |
| **Lock worktrees on portable drives** | `git worktree lock ../usb-work --reason "USB drive"` |
| **Use `repair` after manual moves** | Fixes broken links |

#### Pitfalls to Avoid

| Pitfall | Solution |
|---------|----------|
| Same branch in multiple worktrees | Git blocks this—use different branches |
| Duplicated build artifacts | Share configs where possible; clean build dirs |
| Submodule issues | Experimental support—avoid multiple superproject checkouts |
| Manual directory deletion | Always use `git worktree remove`, then `prune` |
| Stale worktrees accumulating | Regular `git worktree list` and cleanup |
| Bare repo fetch problems | Configure remote.origin.fetch manually |

#### Shell Aliases (Recommended)

```bash
# Add to ~/.bashrc or ~/.zshrc
alias gwa='git worktree add'
alias gwl='git worktree list'
alias gwr='git worktree remove'
alias gwp='git worktree prune'

# Quick worktree with new branch
gwab() { git worktree add -b "$1" "../$1" "${2:-HEAD}"; }
```

---

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
# Project-specific tests and lints (see Layer 4)
# ...

# Universal quality gates (pmat)
pmat analyze tdg                           # Check technical debt grade
pmat tdg check-regression --baseline .pmat/baseline.json  # If baseline exists
pmat mutate --target <changed-files> --threshold 80       # Mutation testing

# File P0 issues for any failures
bd create "Quality gate failure: TDG grade dropped to C" \
  -t bug -p 0 \
  --description="pmat analyze tdg shows grade C, was B. Details: ..." \
  --label quality-gate-failure \
  --json
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

# VERIFY: Must show "up to date with origin"
git status
```

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
│   │   ├── lib-mcp/             # MCP tools (45)
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
pmat analyze tdg --fail-on-violation --min-grade B
```

### Pre-commit Hooks (cargo-husky)

Rust-native pre-commit hooks auto-install on first `cargo test`:

```bash
# Auto-installed to .git/hooks/pre-commit via cargo-husky
# Runs:
#   1. cargo fmt --check (BLOCKING - prevents unformatted commits)
#   2. cargo clippy (advisory - shows warnings)
#   3. cargo audit (advisory - shows vulnerabilities)
#   4. bd sync (beads tracking)

# Manual install
make install-hooks

# Or run tests (auto-installs)
cargo test
```

### Active Work Areas

Current work is tracked in Beads. Always check for latest:

```bash
# Find ready (unblocked) work
bd ready

# See all open issues
bd list --status open

# Current epic
bd show 3gs  # Production Hardening
```

Use `bd` commands to find current priorities rather than relying on static lists.

### MCP Tools Reference (45 tools)

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

**Automatic (cargo-husky)**: Hooks auto-install on `cargo test`:
- `cargo fmt --check` (blocking)
- `cargo clippy` (advisory)
- `cargo audit` (advisory)
- `bd sync` (beads)

**Manual full check**:
```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test -p lib-core --test integration -- --test-threads=1
pmat analyze tdg --fail-on-violation --min-grade B
```

**Install hooks manually**: `make install-hooks` or `cargo test`

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
| vc | See vc repo | `vc --version` |
| pmat | `cargo install pmat` | `pmat --version` |
| mcp-agent-mail | `cargo install --path crates/services/mcp-agent-mail` | `mcp-agent-mail --version` |
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
| vc | `vc --help`, `docs/CONFIGURATION.md`, `docs/FEATURES.md` |
| pmat | `pmat --help`, [PMAT Book](https://paiml.github.io/pmat-book/) |

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
| Issues not syncing | Run `bd sync` and `bd hooks install` |
| Test DB conflicts | Use `--test-threads=1` for integration tests |
| bd in worktree fails | Use `bd --no-daemon` flag |

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

# Quality grade
pmat analyze tdg
```

---

## 📝 DOCUMENT MAINTENANCE

This document should be updated when:
- New tools are added to the project
- Workflow changes are made
- Common issues are discovered
- Project-specific sections need updating

**Version**: 1.1.0
**Last Updated**: 2025-12-17
**Maintainer**: Avyukth

---

## 🛠️ CLAUDE CODE SKILLS REFERENCE

Skills are modular knowledge bases that Claude loads when needed. They provide domain-specific guidelines, best practices, and code examples.

**Location**: `~/.claude/skills/` (global) or `.tmp/skills/` (project export)

### Available Skills

| Skill | Purpose | Use When |
|-------|---------|----------|
| **rust-skills** | Rust backend development with Axum, SQLx, error handling | Writing Rust code, Axum routes, database access |
| **backend-dev-guidelines** | Node.js/Express/TypeScript patterns | Creating API routes, controllers, services |
| **frontend-dev-guidelines** | React/TypeScript/MUI v7 patterns | Creating React components, MUI styling |
| **production-hardening-backend** | Rust backend security, NIST SP 800-53 | Hardening production services, security review |
| **production-hardening-frontend** | SvelteKit security, CSP, Core Web Vitals | Frontend security, performance optimization |
| **kaizen-solaris-review** | Rust code review with Toyota Way philosophy | Code reviews, quality gates |
| **paiml-mcp-toolkit** | PMAT, TDG, Rust Project Score | Code quality analysis, technical debt |
| **git-workflow-mastery** | Git branching, conventional commits | Git operations, PR workflows |
| **c4-architecture** | C4 Model architecture diagrams | System design, architecture docs |
| **deploy-pulumi-argocd-canary** | Kubernetes deployment, GitOps | Infrastructure, deployments |
| **error-tracking** | Sentry v8 error tracking | Adding error handling, monitoring |
| **prd** | Product Requirements Documents | Creating PRDs, feature specs |
| **task-master-prompts** | AI task management prompts | Task breakdown, project management |
| **mobile-frontend-design** | PWA, responsive design | Mobile-first development |
| **sveltekit-pwa-skills** | SvelteKit PWA patterns | Building PWAs with SvelteKit |
| **skill-developer** | Creating new skills | Building custom skills |
| **meta-skill** | Skill architecture guide | Understanding skill system |
| **route-tester** | API route testing with JWT | Testing authenticated endpoints |
| **reasoning-planner** | Planning and reasoning patterns | Complex task planning |

### Skill Activation

Skills auto-activate based on:
- **Keywords** in prompts (e.g., "rust", "backend", "deploy")
- **File patterns** (e.g., editing `*.rs` files triggers rust-skills)
- **Content patterns** (e.g., code containing Axum imports)

### Manual Invocation

```bash
# Invoke a skill directly
/skill rust-skills

# Check skill-rules.json for activation rules
cat ~/.claude/skills/skill-rules.json | jq '.skills["rust-skills"]'
```

### Project Export

Skills exported to `.tmp/skills/` for reference (gitignored).

---

## 📖 RELATED DOCUMENTATION

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — System architecture
- [docs/WALKTHROUGH.md](docs/WALKTHROUGH.md) — Usage walkthrough
- [.env.example](.env.example) — All environment variables
- [scripts/integrations/](scripts/integrations/) — Agent integration configs
- [MCP Protocol](https://modelcontextprotocol.io)
- [PMAT Book](https://paiml.github.io/pmat-book/) — pmat documentation
- [.tmp/skills/README.md](.tmp/skills/README.md) — Skills documentation (local export)

---

*Remember: When in doubt, run `cm context "<your task>"` and `bd ready` to get oriented.*
