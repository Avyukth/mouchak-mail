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

**IMPORTANT**: MCP Agent Mail is available as an **MCP server**. Do NOT treat it as a CLI you must shell out to. If MCP tools are not available, flag to the user — they may need to start the server.

**Server**: `http://localhost:8765`

#### Starting the Server

```bash
# Option 1: Using the 'am' alias (after running: mcp-agent-mail install alias)
am

# Option 2: Direct CLI
mcp-agent-mail serve http

# Option 3: With custom port
mcp-agent-mail serve http --port 9000

# Option 4: MCP stdio mode (for Claude Desktop integration)
mcp-agent-mail serve mcp --transport stdio

# Option 5: MCP SSE mode
mcp-agent-mail serve mcp --transport sse --port 3000
```

#### Server Management

```bash
# Check if server is running
mcp-agent-mail service status --port 8765

# Stop the server
mcp-agent-mail service stop --port 8765

# Restart the server
mcp-agent-mail service restart --port 8765

# Health check
mcp-agent-mail health --url http://localhost:8765
```

#### Robot-Readable Help

```bash
# Get full CLI documentation in JSON (for AI agents)
mcp-agent-mail --robot-help

# Get CLI documentation in YAML
mcp-agent-mail --robot-help --format yaml

# List all MCP tools
mcp-agent-mail tools

# Export tool schemas (JSON or Markdown)
mcp-agent-mail schema --format json
mcp-agent-mail schema --format markdown --output tools.md
```

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

#### API Field Reference

**IMPORTANT**: MCP tools and REST API use slightly different field names:

| Field | MCP Tool | REST API |
|-------|----------|----------|
| Recipients | `to` (comma-separated string) | `recipient_names` (array) |
| CC | `cc` (comma-separated string) | `cc_names` (array) |
| BCC | `bcc` (comma-separated string) | `bcc_names` (array) |
| Agent name | `name` (register_agent) | `name` |
| Project | `project_slug` | `project_slug` |
| Ack Required | `ack_required` (boolean, optional) | `ack_required` (boolean, default: false) |

**`ack_required` Field**: When set to `true`, recipients must explicitly acknowledge the message. Messages with `ack_required=true` appear in the `list_pending_reviews` API until all recipients acknowledge. Use for:
- Task completion reports requiring review
- Handoff requests requiring confirmation
- Code review requests

**Note**: `reply_message` is simpler—it only takes `message_id`, `sender_name`, `body_md`. Recipients and thread are auto-derived from the original message.

#### Setup Workflow

**Step 1: Start Server (if not running)**

```bash
# Using alias (recommended)
am

# Or direct CLI
mcp-agent-mail serve http --port 8765
```

**Step 2: Register Identity (via MCP tools)**

Use the `ensure_project` and `register_agent` MCP tools:

| Tool | Parameters |
|------|------------|
| `ensure_project` | `slug`: absolute repo path (e.g., `/Users/me/myrepo`)<br>`human_key`: friendly name (e.g., `my-project`) |
| `register_agent` | `project_slug`: from ensure_project<br>`name`: unique agent name<br>`program`: e.g., `claude-code`<br>`model`: e.g., `claude-opus-4`<br>`task_description`: what this agent does |

**Step 3: Reserve Files Before Editing**

Use `file_reservation_paths` to prevent conflicts:

| Parameter | Description |
|-----------|-------------|
| `project_slug` | Project from step 2 |
| `agent_name` | Your agent name |
| `paths` | Array of glob patterns: `["src/**/*.rs", "Cargo.toml"]` |
| `ttl_seconds` | Reservation duration (e.g., `3600` for 1 hour) |
| `exclusive` | `true` for write access, `false` for read-only |

**Step 4: Communicate via Threads**

Use `send_message` for async communication:

| Parameter | Description |
|-----------|-------------|
| `project_slug` | Project identifier |
| `sender_name` | Your agent name |
| `to` | Recipient(s), comma-separated: `"agent1,agent2"` |
| `subject` | Message subject |
| `body_md` | Markdown message body |
| `thread_id` | Thread identifier (e.g., `"FEAT-123"`) |
| `importance` | `low`, `normal`, `high`, `urgent` (optional) |
| `ack_required` | `true` if acknowledgment needed (optional) |

**Step 5: Check for Messages**

| Tool | Purpose |
|------|---------|
| `check_inbox` | Get unread messages: `agent_name`, `unread_only=true` |
| `acknowledge_message` | Mark message acknowledged: `message_id`, `agent_name` |
| `mark_message_read` | Mark as read without ack: `message_id`, `agent_name` |

**Step 6: Release Reservations**

Use `release_reservation` with `reservation_id` when done editing files.

#### REST API Reference (Alternative to MCP)

> **Important**: Most endpoints use POST with JSON body. Use correct HTTP method.

##### Health & Status
```bash
# Health check (GET)
curl http://localhost:8765/api/health

# List all projects (GET)
curl http://localhost:8765/api/projects
```

##### Agent Management
```bash
# Register agent (POST)
curl -X POST http://localhost:8765/api/agent/register \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project","name":"worker-1","program":"claude-code","model":"claude-3.5-sonnet"}'

# List agents for project (GET)
curl http://localhost:8765/api/projects/my-project/agents

# Whois lookup (POST)
curl -X POST http://localhost:8765/api/agent/whois \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project","agent_name":"worker-1"}'
```

##### Messaging
```bash
# Send message (POST)
curl -X POST http://localhost:8765/api/message/send \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project","sender_name":"worker-1","recipient_names":["reviewer"],"subject":"Test","body_md":"Hello","importance":"normal"}'

# Check inbox (POST - not GET!)
curl -X POST http://localhost:8765/api/inbox \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project","agent_name":"worker-1","limit":20}'

# Get single message by ID (GET with path param)
curl http://localhost:8765/api/messages/123

# Get thread (POST)
curl -X POST http://localhost:8765/api/thread \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project","thread_id":"TASK-123"}'

# Mark message read (POST)
curl -X POST http://localhost:8765/api/message/read \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project","message_id":123,"agent_name":"worker-1"}'

# Unified inbox - all projects (GET with query params)
curl "http://localhost:8765/mail/api/unified-inbox?importance=high&limit=50"
```

##### File Reservations
```bash
# Reserve files (POST)
curl -X POST http://localhost:8765/api/file_reservations/paths \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project","agent_name":"worker-1","file_paths":["src/main.rs"]}'

# List reservations (POST)
curl -X POST http://localhost:8765/api/file_reservations/list \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project"}'

# Release reservation (POST)
curl -X POST http://localhost:8765/api/file_reservations/release \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project","reservation_id":1}'
```

##### Project Info
```bash
# Get project info (POST)
curl -X POST http://localhost:8765/api/project/info \
  -H "Content-Type: application/json" \
  -d '{"project_slug":"my-project"}'

# Ensure project exists (POST - creates if missing)
curl -X POST http://localhost:8765/api/project/ensure \
  -H "Content-Type: application/json" \
  -d '{"human_key":"my-project"}'
```


#### MCP Resources (resource:// URIs)

The `resource://` scheme provides read-only access to data. Supports lazy loading via `include_bodies` parameter.

| Resource URI | Description | Query Params |
|--------------|-------------|--------------|
| `resource://inbox/{agent}?project={slug}` | Agent's inbox messages | `limit`, `include_bodies` |
| `resource://outbox/{agent}?project={slug}` | Agent's sent messages | `limit`, `include_bodies` |
| `resource://thread/{id}?project={slug}` | Messages in a thread | `include_bodies` |
| `resource://threads?project={slug}` | List all threads | `limit` |
| `resource://agent/{name}?project={slug}` | Single agent info | — |
| `resource://agents?project={slug}` | All agents in project | — |
| `resource://file_reservations?project={slug}` | Active file reservations | — |
| `resource://product/{uid}` | Product info (no project needed) | — |
| `resource://identity/{path}` | Repository identity (no project needed) | — |

**Query Parameters:**
- `project` — Project slug (required for most resources)
- `limit` — Max results (default: 20)
- `include_bodies` — Include message bodies (`true`/`false`, default: `false`)

**Lazy Loading:** By default, inbox/outbox/thread resources omit `body_md` for token efficiency. Set `include_bodies=true` to include full message bodies.

**Legacy Scheme:** `agent-mail://{project}/{resource}/{id}` still supported for backwards compatibility.

#### Pre-Commit Guard

The pre-commit guard prevents commits that conflict with file reservations. Install via MCP tools (`install_precommit_guard`) or configure behavior via environment variables.

**Environment Variables:**

| Variable | Values | Description |
|----------|--------|-------------|
| `AGENT_NAME` | string | Your agent identity for reservation checks |
| `AGENT_MAIL_BYPASS` | `1` | Skip all guard checks (emergency only) |
| `AGENT_MAIL_GUARD_MODE` | `enforce` (default) | Block conflicting commits |
| | `warn` / `advisory` | Warn but allow commits |
| `WORKTREES_ENABLED` | `1` | Enable worktree-aware features |
| `GIT_IDENTITY_ENABLED` | `1` | Enable git identity features |

#### Archive & Disaster Recovery

```bash
# Create a backup archive
mcp-agent-mail archive save --label "pre-refactor"
mcp-agent-mail archive save --include-git  # Include git storage

# List available restore points
mcp-agent-mail archive list
mcp-agent-mail archive list --json

# Restore from archive
mcp-agent-mail archive restore data/archives/archive_pre-refactor_20250101_120000.zip
mcp-agent-mail archive restore <file> --yes  # Skip confirmation

# Clear all data (creates backup first if --archive)
mcp-agent-mail archive clear-and-reset --archive --label "pre-wipe"
mcp-agent-mail archive clear-and-reset --yes  # Skip confirmation (DESTRUCTIVE)
```

#### Share & Export

```bash
# Generate signing keypair
mcp-agent-mail share keypair
mcp-agent-mail share keypair --output keys.json

# Verify export manifest signature
mcp-agent-mail share verify --manifest export_manifest.json
mcp-agent-mail share verify --manifest export_manifest.json --public-key <key>

# Encrypt/decrypt (placeholder - use Python version for now)
mcp-agent-mail share encrypt --project my-project --passphrase <pass>
mcp-agent-mail share decrypt --input file.age --passphrase <pass>
```

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

**Version**: 1.2.0
**Last Updated**: 2025-12-21
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

## 🤖 LAYER 5: MULTI-AGENT ORCHESTRATION

This layer defines the workflow for autonomous multi-agent task execution with review and human oversight.

### Agent Roles

| Role | Agent Name | Responsibility |
|------|------------|----------------|
| **Worker** | `worker-<id>` | Claims tasks from beads, implements code, runs quality gates |
| **Reviewer** | `reviewer` | Deep code review, logic validation, gap analysis, fixes issues |
| **Human** | `human` | Final oversight, receives completion reports |

#### Reviewer Agent Detailed Responsibilities

The Reviewer is the quality gatekeeper. It performs **deep analysis** of all implementations:

**1. Code Block Analysis**
- Read every changed file completely
- Verify logic flow matches acceptance criteria
- Check for `todo!()`, `unimplemented!()`, placeholder comments
- Verify error handling uses `Result<T, E>` not `unwrap()`/`expect()`
- Check function complexity (cognitive complexity < 30)

**2. Implementation Completeness**
- Compare beads acceptance criteria vs actual code
- Identify missing edge cases
- Verify all paths are tested
- Check for hardcoded values that should be configurable

**3. Logic Validation**
- Trace data flow through the implementation
- Verify business logic correctness
- Check boundary conditions
- Validate error messages are actionable

**4. Enhancement Gap Analysis**
- Identify opportunities for improvement
- Note technical debt introduced
- Flag potential performance issues
- Suggest refactoring if needed

**5. Critical Analysis**
- Security: SQL injection, path traversal, input validation
- Concurrency: race conditions, deadlocks
- Resource leaks: file handles, connections
- API contracts: breaking changes, backward compatibility

**6. UI/Frontend Validation** (when PR modifies UI components, styles, or user-facing features)

Use **dev-browser** (not Playwright MCP) for token efficiency (39% cheaper, 43% fewer turns):

```bash
# Install dev-browser plugin (one-time)
/plugin marketplace add sawyerhood/dev-browser
/plugin install dev-browser@sawyerhood/dev-browser
```

**Validation Checklist:**

| Category | What to Check | Tool |
|----------|---------------|------|
| **Navigation** | All routes render, links work, back/forward | dev-browser |
| **Components** | Buttons clickable, forms submit, modals open/close | dev-browser |
| **Responsive** | Mobile (375px), tablet (768px), desktop (1280px) | dev-browser resize |
| **Accessibility** | Focus order, ARIA labels, keyboard navigation | dev-browser + manual |
| **Visual** | Layout not broken, text readable, icons visible | Screenshot compare |

**Quick Validation Script:**

```typescript
// dev-browser script for UI smoke test
await page.goto('http://localhost:8765');
await page.waitForSelector('[data-testid="app-loaded"]');

// Navigation check
await page.click('[data-testid="nav-inbox"]');
await expect(page.url()).toContain('/inbox');

// Responsive check
await page.setViewportSize({ width: 375, height: 667 });
await page.screenshot({ path: 'mobile.png' });
```

**Reference Skills** (for detailed patterns):
- `~/.claude/skills/frontend-dev-guidelines/` - Component patterns, styling
- `~/.claude/skills/production-hardening-frontend/` - CSP, XSS prevention
- `~/.claude/skills/mobile-frontend-design/` - Touch, responsive, PWA

### Workflow Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    MULTI-AGENT ORCHESTRATION FLOW                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌──────────┐  │
│  │   BEADS     │────▶│   WORKER    │─ ─ ▶│  REVIEWER   │────▶│  HUMAN   │  │
│  │  (bd ready) │     │   AGENT     │     │   AGENT     │     │  AGENT   │  │
│  └─────────────┘     └──────┬──────┘     └─────────────┘     └──────────┘  │
│                             │                   │                          │
│                             │ [COMPLETION]      │ [APPROVED/FIXED]         │
│                             │ mail (async)      │ mail to human            │
│                             ▼                   ▼                          │
│                      ┌─────────────┐     ┌─────────────┐                   │
│                      │  SESSION    │     │ Git Worktree│                   │
│                      │    ENDS     │     │ (fix if bad)│                   │
│                      │ (no await)  │     └─────────────┘                   │
│                      └─────────────┘                                        │
│                                                                             │
│  KEY: Worker sends [COMPLETION] and EXITS. Does NOT wait for [APPROVED].   │
│       Reviewer picks up async. Single-agent fallback sends direct to Human.│
└─────────────────────────────────────────────────────────────────────────────┘
```

### Phase 1: Task Claim (Worker Agent)

**Step 1-2: Find and claim work (CLI)**

```bash
# Find available work
bd ready --json

# Claim the task
bd update <task-id> --status in_progress --json
```

**Step 3: Register as worker agent (MCP tools)**

| Tool | Parameters |
|------|------------|
| `ensure_project` | `slug`: repo absolute path<br>`human_key`: `"mcp-agent-mail-rs"` |
| `register_agent` | `project_slug`: from ensure_project<br>`name`: `"worker-<task-id>"`<br>`program`: `"claude-code"`<br>`model`: `"claude-opus-4"`<br>`task_description`: `"Working on task <task-id>: <task-title>"` |

**Step 4: Reserve files for exclusive access (MCP tool)**

| Tool | Parameters |
|------|------------|
| `file_reservation_paths` | `project_slug`: from step 3<br>`agent_name`: `"worker-<task-id>"`<br>`paths`: `["src/**/*.rs", "Cargo.toml"]` (based on task scope)<br>`ttl_seconds`: `7200` (2 hours)<br>`exclusive`: `true` |

**Step 5: (Optional) Notify reviewer (MCP tool)**

| Tool | Parameters |
|------|------------|
| `send_message` | `project_slug`: from step 3<br>`sender_name`: `"worker-<task-id>"`<br>`to`: `"reviewer"`<br>`subject`: `"[TASK_STARTED] <task-id>: <task-title>"`<br>`body_md`: `"Starting work on task. ETA: <estimate>"`<br>`thread_id`: `"TASK-<task-id>"` |

### Phase 2: Implementation (Worker Agent)

```bash
# 1. Create isolated worktree
git worktree add .sandboxes/worker-<task-id> -b feature/<task-id>
cd .sandboxes/worker-<task-id>

# 2. Implement based on beads acceptance criteria
# ... coding work ...

# 3. Run quality gates
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test -p lib-core --test integration -- --test-threads=1

# 4. Commit changes
git add -A
git commit -m "feat(<scope>): <description>

Implements: <task-id>
- <change 1>
- <change 2>

🤖 Generated with Claude Code"

# 5. Return to main and merge
cd ../..
git checkout main
git merge feature/<task-id> --no-edit

# 6. Clean up worktree
git worktree remove .sandboxes/worker-<task-id>
git branch -d feature/<task-id>
```

### Phase 3: Completion Mail (Worker → Reviewer)

**Mail Subject Format**: `[COMPLETION] <task-id>: <task-title>`

**Required Fields**:

```markdown
## Task Completion Report

**Task ID**: <beads-id>
**Task Title**: <title from beads>
**Commit ID**: <40-char git SHA>
**Branch**: main (merged from feature/<task-id>)

### Files Changed
<output of: git diff --name-only <merge-base>..HEAD>

### Summary of Changes
<2-3 sentence summary of what was implemented>

### Acceptance Criteria Status
- [x] Criterion 1 from beads
- [x] Criterion 2 from beads
- [ ] Criterion 3 (partial - see notes)

### Quality Gates
| Gate | Status | Notes |
|------|--------|-------|
| cargo check | ✅ PASS | |
| cargo clippy | ✅ PASS | 0 warnings |
| cargo fmt | ✅ PASS | |
| cargo test | ✅ PASS | 42 tests |

### Notes for Reviewer
<any context, caveats, or areas needing attention>

### Verification Command
```bash
git show <commit-id> --stat
cargo test -p <affected-crate>
```
```

**MCP Tool: `send_message`**

| Parameter | Value |
|-----------|-------|
| `project_slug` | `"<project-slug>"` |
| `sender_name` | `"worker-<task-id>"` |
| `to` | `"reviewer"` (or `"human"` if no reviewer; comma-separated for multiple) |
| `cc` | `"human"` (human CCed for audit trail) |
| `subject` | `"[COMPLETION] <task-id>: <task-title>"` |
| `body_md` | `"<markdown report above>"` |
| `thread_id` | `"TASK-<task-id>"` |
| `importance` | `"high"` (options: `low`, `normal`, `high`, `urgent`) |
| `ack_required` | `true` (requires reviewer acknowledgment) |

**Note**: Human is CCed so they see all completions even before review.

#### Worker Session Complete

**IMPORTANT**: After sending [COMPLETION] mail, the Worker Agent's job is DONE.

```
┌─────────────────────────────────────────────────────────────┐
│  WORKER LIFECYCLE (Fire-and-Forget)                        │
├─────────────────────────────────────────────────────────────┤
│  1. Claim task (bd update --status in_progress)            │
│  2. Register agent                                          │
│  3. Reserve files                                          │
│  4. Implement & run quality gates                          │
│  5. Commit & merge to main                                 │
│  6. Send [COMPLETION] mail                                 │
│  7. Release reservations                                   │
│  8. ✅ SESSION ENDS — Worker does NOT await [APPROVED]     │
└─────────────────────────────────────────────────────────────┘
```

The worker:
- Does NOT wait for reviewer [APPROVED] message
- Does NOT poll inbox for review results
- Releases all file reservations after sending completion mail
- Session terminates cleanly

The Reviewer Agent asynchronously picks up [COMPLETION] mails and handles review.

---

### Phase 4: Review (Reviewer Agent)

#### 4.1 Check Inbox and State

**Step 1: Check for completion mails**

Use MCP tool `check_inbox`:
| Parameter | Value |
|-----------|-------|
| `project_slug` | `"<project-slug>"` |
| `agent_name` | `"reviewer"` |
| `unread_only` | `true` |

Filter results for messages where subject contains `[COMPLETION]`.

**Step 2: For each completion mail, check thread state**

Use MCP tool `list_threads` or `summarize_thread` with `thread_id` from the mail.

**Decision Logic:**

| Thread Contains | Action |
|-----------------|--------|
| `[APPROVED]` or `[FIXED]` | Skip — already reviewed |
| `[REVIEWING]` | Skip — another reviewer claimed it |
| Only `[COMPLETION]` | Proceed — claim and review |

#### 4.2 Validation Checklist

The Reviewer MUST perform comprehensive validation:

##### Step 1: Fetch and Read Changed Files

```bash
# Get list of changed files from completion mail
git diff --name-only <merge-base>..HEAD

# Read each changed file completely
for file in $(git diff --name-only <merge-base>..HEAD); do
  cat "$file"  # Reviewer reads entire file
done
```

##### Step 2: Placeholder Detection

```bash
# Search for placeholder patterns (MUST be zero matches)
rg -n "todo!|unimplemented!|FIXME|XXX|HACK|placeholder" <changed-files>
rg -n "// TODO|# TODO|/* TODO" <changed-files>
rg -n "panic!|unwrap\(\)|expect\(" <changed-files>  # Should use Result
```

| Pattern | Severity | Action |
|---------|----------|--------|
| `todo!()` | CRITICAL | Must implement |
| `unimplemented!()` | CRITICAL | Must implement |
| `unwrap()` in src/ | HIGH | Convert to `?` or `ok_or()` |
| `expect()` without justification | MEDIUM | Add comment or use `?` |
| `// TODO` comments | MEDIUM | Implement or create beads task |
| `FIXME` / `XXX` / `HACK` | LOW | Document or fix |

##### Step 3: Logic Validation

For each function in changed files:

```
1. Read function signature
2. Trace input → processing → output
3. Verify:
   - [ ] All code paths return valid values
   - [ ] Error cases handled (not ignored)
   - [ ] Edge cases covered (empty, null, max values)
   - [ ] Boundary conditions tested
   - [ ] Side effects documented
```

##### Step 4: Acceptance Criteria Mapping

```markdown
## Criteria Verification

| Beads Criterion | Code Location | Status | Notes |
|-----------------|---------------|--------|-------|
| Criterion 1 | file.rs:42-60 | ✅ | Fully implemented |
| Criterion 2 | file.rs:70-85 | ⚠️ | Missing edge case |
| Criterion 3 | NOT FOUND | ❌ | Not implemented |
```

##### Step 5: Quality Gate Re-run

```bash
# MUST pass all gates
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --workspace --exclude e2e-tests
```

##### Step 6: Critical Analysis

| Category | What to Check | Tools |
|----------|---------------|-------|
| **Security** | SQL injection, XSS, path traversal | `cargo audit`, manual review |
| **Concurrency** | Race conditions, deadlocks | Look for `Mutex`, `RwLock`, `async` |
| **Resources** | File handle leaks, connection pools | Check `Drop` impls, RAII patterns |
| **Performance** | O(n²) loops, unnecessary clones | `cargo clippy`, manual review |
| **API** | Breaking changes, deprecations | Compare with previous API |

##### Step 7: Gap Analysis

Document any gaps found:

```markdown
## Enhancement Gaps Identified

### Must Fix (Blocking)
- [ ] Missing error handling in X
- [ ] Placeholder code in Y

### Should Fix (Non-blocking)
- [ ] Could optimize Z
- [ ] Consider adding config for W

### Future Work (Create beads)
- [ ] Refactor: <description>
- [ ] Enhancement: <description>
```

##### Summary: Pass/Fail Criteria

| Check | Pass Criteria |
|-------|--------------|
| **Placeholder-Free** | Zero `todo!()`, `unimplemented!()` in src/ |
| **Acceptance Criteria** | 100% criteria mapped to code |
| **Quality Gates** | All cargo commands exit 0 |
| **Logic Correctness** | No obvious bugs in traced paths |
| **Security** | No OWASP Top 10 vulnerabilities |
| **Resources** | No leaks identified |

#### 4.3 If Review PASSES

**Step 1: Claim the review**

Use MCP tool `reply_message` (auto-sends to original sender, same thread):

| Parameter | Value |
|-----------|-------|
| `project_slug` | `"<project-slug>"` |
| `message_id` | `<completion-mail-id>` |
| `sender_name` | `"reviewer"` |
| `body_md` | `"[REVIEWING] Review claimed. Starting validation..."` |

**Step 2: Perform validation** (steps 1-7 from checklist)

**Step 3: Send approval**

Use MCP tool `send_message` (for CC support):

| Parameter | Value |
|-----------|-------|
| `project_slug` | `"<project-slug>"` |
| `sender_name` | `"reviewer"` |
| `to` | `"worker-<task-id>"` |
| `cc` | `"human"` |
| `subject` | `"[APPROVED] <task-id>: <task-title>"` |
| `body_md` | (see template below) |
| `thread_id` | `"TASK-<task-id>"` |

**Approval body template:**

```markdown
## Review Complete - APPROVED

**Task ID**: <task-id>
**Commit ID**: <commit-sha>
**Worker**: worker-<task-id>

### Verification Summary
- Implementation is complete (not placeholder)
- All acceptance criteria met
- Quality gates passed
- Code follows project standards

### Files Changed
<list of files>

No issues found. Ready for production.
```

**Step 4: Close beads task**

```bash
bd close <task-id> --reason "Implementation verified by reviewer"
```

**Step 5-6: Acknowledge and mark as read**

| Tool | Parameters |
|------|------------|
| `acknowledge_message` | `project_slug`, `message_id`: `<completion-mail-id>`, `agent_name`: `"reviewer"` |
| `mark_message_read` | `project_slug`, `message_id`: `<completion-mail-id>`, `agent_name`: `"reviewer"` |

#### 4.4 If Review FAILS (Fix Flow)

```bash
# 1. Create review worktree
git worktree add .sandboxes/reviewer-fix-<task-id> -b fix/<task-id>
cd .sandboxes/reviewer-fix-<task-id>

# 2. Fix identified issues
# ... fix placeholder code, missing implementations, etc. ...

# 3. Run quality gates
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test

# 4. Commit fixes
git add -A
git commit -m "fix(<scope>): reviewer fixes for <task-id>

Fixes:
- <issue 1>
- <issue 2>"

# 5. Merge to main
cd ../..
git checkout main
git merge fix/<task-id> --no-edit

# 6. Clean up
git worktree remove .sandboxes/reviewer-fix-<task-id>
git branch -d fix/<task-id>
```

**Then send fix completion mail:**

Use MCP tool `send_message`:

| Parameter | Value |
|-----------|-------|
| `project_slug` | `"<project-slug>"` |
| `sender_name` | `"reviewer"` |
| `to` | `"worker-<task-id>"` |
| `cc` | `"human"` |
| `subject` | `"[FIXED] <task-id>: <task-title>"` |
| `body_md` | (see template below) |
| `thread_id` | `"TASK-<task-id>"` |

**Fixed body template:**

```markdown
## Review Complete - FIXED

**Task ID**: <task-id>
**Original Commit**: <worker-commit-sha>
**Fix Commit**: <reviewer-fix-sha>
**Worker**: worker-<task-id>

### Issues Found
1. <issue description>
2. <issue description>

### Fixes Applied
1. <fix description>
2. <fix description>

### Final Status
- All acceptance criteria now met
- Quality gates passing
- Ready for production

### Commits
- `<worker-sha>` - Original implementation
- `<fix-sha>` - Reviewer fixes
```

**Close beads task:**

```bash
bd close <task-id> --reason "Fixed by reviewer"
```

### Phase 5: Human Notification

Human agent receives final report and can:

1. **Acknowledge** - Task complete, no further action
2. **Request Changes** - Send message back to reviewer/worker
3. **Close Loop** - Mark beads task as done

**Human actions:**

| Action | Tool/Command |
|--------|--------------|
| Acknowledge | MCP `acknowledge_message`: `project_slug`, `message_id`, `agent_name`: `"human"` |
| Close task | `bd close <task-id> --reason "Verified by human"` |

### Single-Agent Fallback Mode

When **no Reviewer agent is present**, Worker performs self-review:

**Step 1: Check if reviewer exists**

Use MCP tool `list_agents` with `project_slug`. Check if any agent has `name == "reviewer"`.

**Step 2: If no reviewer, self-review and send directly to Human**

1. Re-run quality gates
2. Verify acceptance criteria
3. Send directly to Human using `send_message`:

| Parameter | Value |
|-----------|-------|
| `project_slug` | `"<project-slug>"` |
| `sender_name` | `"worker-<task-id>"` |
| `to` | `"human"` (skip reviewer, go direct) |
| `subject` | `"[COMPLETION] <task-id>: <task-title> (Self-Reviewed)"` |
| `thread_id` | `"TASK-<task-id>"` |

**Self-reviewed completion body:**

```markdown
## Task Completion Report (Self-Reviewed)

**Note**: No reviewer agent present. Worker performed self-review.

<... standard completion report ...>

### Self-Review Checklist
- [x] Implementation is not placeholder
- [x] Acceptance criteria met
- [x] Quality gates passed
- [x] Code follows project standards
```

### Agent Registration Protocol

At session start, each agent type registers with ALL required fields:

**Required Fields for `register_agent`:**
| Field | Type | Description |
|-------|------|-------------|
| `project_slug` | string | Project slug (URL-safe identifier, e.g., `users-amrit-documents-projects-rust-mouchak-mcp-agent-mail-rs`) |
| `name` | string | Agent name (unique within project) |
| `program` | string | Program identifier (e.g., `claude-code`, `antigravity`) |
| `model` | string | Model being used (e.g., `claude-opus-4`, `claude-sonnet-4`) |
| `task_description` | string | Description of agent's task/responsibilities |

**Agent registration examples (via MCP `register_agent`):**

| Agent Type | `name` | `program` | `model` | `task_description` |
|------------|--------|-----------|---------|-------------------|
| Worker | `"worker-<session-id>"` | `"claude-code"` | `"claude-opus-4"` | `"Implements features and fixes based on beads tasks"` |
| Reviewer | `"reviewer"` | `"claude-code"` | `"claude-opus-4"` | `"Reviews implementations, validates code quality, fixes issues"` |
| Human | `"human"` | `"human-operator"` | `"human"` | `"Final oversight and approval of completed work"` |

### Message Thread Conventions

| Thread ID Pattern | Purpose |
|-------------------|---------|
| `TASK-<beads-id>` | All messages about a specific task |
| `REVIEW-<date>` | Daily review summaries |
| `ESCALATE-<id>` | Issues needing human attention |
| `SYSTEM` | Infrastructure/tooling messages |

### Review State Tracking (Thread-Based)

**Problem**: How does Reviewer know if a task has already been reviewed?

**Solution**: Use threads as state machines. All task messages share `thread_id="TASK-<beads-id>"`. Check thread history before acting.

**Note**: This state machine is for REVIEWER use only. Workers do NOT poll this—they send [COMPLETION] and exit.

#### State Machine (via Subject Prefixes)

```
[TASK_STARTED] → [COMPLETION] → [REVIEWING] → [APPROVED|REJECTED|FIXED] → [ACK]
     ↑              ↑                ↑                    ↑                  ↑
   Worker        Worker          Reviewer             Reviewer            Human
```

| State | Subject Prefix | Sender | Meaning |
|-------|---------------|--------|---------|
| `STARTED` | `[TASK_STARTED]` | Worker | Work begun |
| `COMPLETED` | `[COMPLETION]` | Worker | Ready for review |
| `CLAIMED` | `[REVIEWING]` | Reviewer | Review in progress (prevents duplicates) |
| `APPROVED` | `[APPROVED]` | Reviewer | Passed review |
| `REJECTED` | `[REJECTED]` | Reviewer | Failed, needs rework |
| `FIXED` | `[FIXED]` | Reviewer | Failed but fixed by reviewer |
| `ACKNOWLEDGED` | `[ACK]` | Human | Human confirmed |

#### Check Thread State Before Review

**Step 1:** Use MCP tool `list_threads` or `summarize_thread` with `thread_id="TASK-<task-id>"`.

**Step 2:** Parse thread message subjects to determine state:

| Subject Contains | State | Decision |
|------------------|-------|----------|
| `[APPROVED]` or `[FIXED]` | Already reviewed | Skip — task complete |
| `[REVIEWING]` | Claimed by other | Skip — another reviewer has it |
| Only `[COMPLETION]` | Ready | Proceed — claim and review |

#### Review Claim Protocol (Prevent Duplicates)

Before starting review, Reviewer sends a `[REVIEWING]` message to claim the task:

**Step 1: Claim the review**

Use MCP tool `reply_message`:

| Parameter | Value |
|-----------|-------|
| `project_slug` | `"<project-slug>"` |
| `message_id` | `<completion-mail-id>` |
| `sender_name` | `"reviewer"` |
| `body_md` | `"[REVIEWING] Review claimed by reviewer. Starting validation..."` |

**Step 2:** Perform review (validation checklist from 4.2)

**Step 3: Send result with CC**

Use MCP tool `send_message` (for CC support):

| Parameter | Value |
|-----------|-------|
| `project_slug` | `"<project-slug>"` |
| `sender_name` | `"reviewer"` |
| `to` | `"worker-<task-id>"` |
| `cc` | `"human"` |
| `subject` | `"[APPROVED] <task-id>: <task-title>"` |
| `body_md` | `"<review results>"` |
| `thread_id` | `"TASK-<task-id>"` |

#### CC/BCC Usage for Audit Trail

| Field | Recipients | Purpose |
|-------|------------|---------|
| **To** | Primary actor (worker or reviewer) | Direct communication |
| **CC** | `human` | Audit trail - human sees all state changes |
| **BCC** | `metrics` (optional) | Analytics without cluttering thread |

**CC Rules:**
- Worker `[COMPLETION]` → CC: `human` (human knows work done)
- Reviewer `[REVIEWING]` → CC: `human` (human knows review started)
- Reviewer `[APPROVED/REJECTED/FIXED]` → To: `worker`, CC: `human`
- Human `[ACK]` → To: `reviewer`, CC: `worker`

#### Complete Message Chain Example

```
Thread: TASK-abc123
─────────────────────────────────────────────────────────────────

[1] From: worker-abc123
    To: reviewer
    CC: human
    Subject: [COMPLETION] abc123: Add input validation
    Body: ## Task Completion Report...

    [2] From: reviewer
        To: worker-abc123
        CC: human
        Subject: [REVIEWING] abc123: Add input validation
        Body: Review claimed. Starting validation...

        [3] From: reviewer
            To: worker-abc123
            CC: human
            Subject: [APPROVED] abc123: Add input validation
            Body: ## Review Complete - APPROVED...

            [4] From: human
                To: reviewer
                CC: worker-abc123
                Subject: [ACK] abc123: Add input validation
                Body: Acknowledged. Closing task.
```

#### Query Review History

| Task | MCP Tool | Key Parameters |
|------|----------|----------------|
| Full review history | `list_threads` | `project_slug`, `thread_id="TASK-<task-id>"`, `include_bodies=true` |
| Quick status | `summarize_thread` | `project_slug`, `thread_id="TASK-<task-id>"` |
| All reviewed tasks | `search_messages` | `project_slug`, `query="[APPROVED] OR [FIXED]"`, `agent_name="reviewer"` |

#### State Transition Rules

| Current State | Valid Next States | Invalid Transitions |
|---------------|-------------------|---------------------|
| (none) | STARTED | - |
| STARTED | COMPLETION | APPROVED, FIXED |
| COMPLETION | REVIEWING | APPROVED (must claim first) |
| REVIEWING | APPROVED, REJECTED, FIXED | COMPLETION (can't go back) |
| REJECTED | COMPLETION (rework) | APPROVED (must re-review) |
| APPROVED | ACK | REJECTED (too late) |
| FIXED | ACK | REJECTED |
| ACK | (terminal) | All (task closed) |

### Environment Variables for Orchestration

| Variable | Default | Description |
|----------|---------|-------------|
| `AGENT_ROLE` | worker | Agent role (worker, reviewer, human) |
| `AGENT_NAME` | auto | Agent identifier |
| `REVIEWER_REQUIRED` | true | Require reviewer before human notification |
| `AUTO_FIX_ENABLED` | true | Allow reviewer to auto-fix issues |
| `WORKTREE_BASE` | .sandboxes | Base directory for agent worktrees |

### Quality Gate Enforcement

All agents MUST run quality gates before any commit:

```bash
# Minimum gates (blocking)
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Extended gates (advisory but logged)
cargo test --workspace
pmat analyze tdg
cargo audit
```

### Error Recovery

| Scenario | Recovery Action |
|----------|-----------------|
| Worker crashes mid-task | Reviewer claims abandoned task, continues in worktree |
| Reviewer crashes mid-review | Human notified, can reassign to new reviewer |
| Git merge conflict | Agent creates conflict resolution commit, notifies in thread |
| Quality gate failure | Task stays open, failure details in completion mail |
| Message delivery failure | Retry with exponential backoff, escalate after 3 failures |

### Orchestration Metrics

Track via `list_tool_metrics()`:

| Metric | Target |
|--------|--------|
| Task claim → completion | < 2 hours |
| Review turnaround | < 30 minutes |
| Fix turnaround | < 1 hour |
| Human acknowledgment | < 24 hours |

---

## 📖 RELATED DOCUMENTATION

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — System architecture
- [docs/WALKTHROUGH.md](docs/WALKTHROUGH.md) — Usage walkthrough
<!-- - [docs/AGENT_COMMUNICATION_PROTOCOL.md](docs/AGENT_COMMUNICATION_PROTOCOL.md) — Worker/reviewer messaging protocol -->
- [.env.example](.env.example) — All environment variables
- [scripts/integrations/](scripts/integrations/) — Agent integration configs
- [MCP Protocol](https://modelcontextprotocol.io)
- [PMAT Book](https://paiml.github.io/pmat-book/) — pmat documentation
- [.tmp/skills/README.md](.tmp/skills/README.md) — Skills documentation (local export)

---

*Remember: When in doubt, run `cm context "<your task>"` and `bd ready` to get oriented.*

<!-- bv-agent-instructions-v1 -->

---

## Beads Workflow Integration

This project uses [beads_viewer](https://github.com/Dicklesworthstone/beads_viewer) for issue tracking. Issues are stored in `.beads/` and tracked in git.

### Essential Commands

```bash
# View issues (launches TUI - avoid in automated sessions)
bv

# CLI commands for agents (use these instead)
bd ready              # Show issues ready to work (no blockers)
bd list --status=open # All open issues
bd show <id>          # Full issue details with dependencies
bd create --title="..." --type=task --priority=2
bd update <id> --status=in_progress
bd close <id> --reason="Completed"
bd close <id1> <id2>  # Close multiple issues at once
bd sync               # Commit and push changes
```

### Workflow Pattern

1. **Start**: Run `bd ready` to find actionable work
2. **Claim**: Use `bd update <id> --status=in_progress`
3. **Work**: Implement the task
4. **Complete**: Use `bd close <id>`
5. **Sync**: Always run `bd sync` at session end

### Key Concepts

- **Dependencies**: Issues can block other issues. `bd ready` shows only unblocked work.
- **Priority**: P0=critical, P1=high, P2=medium, P3=low, P4=backlog (use numbers, not words)
- **Types**: task, bug, feature, epic, question, docs
- **Blocking**: `bd dep add <issue> <depends-on>` to add dependencies

### Session Protocol

**Before ending any session, run this checklist:**

```bash
git status              # Check what changed
git add <files>         # Stage code changes
bd sync                 # Commit beads changes
git commit -m "..."     # Commit code
bd sync                 # Commit any new beads changes
git push                # Push to remote
```

### Best Practices

- Check `bd ready` at session start to find available work
- Update status as you work (in_progress → closed)
- Create new issues with `bd create` when you discover tasks
- Use descriptive titles and set appropriate priority/type
- Always `bd sync` before ending session

<!-- end-bv-agent-instructions -->
