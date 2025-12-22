# Autonomous Agent Initialization Guide (MCP Tools)

> **Purpose**: Enable AI agents to work autonomously on beads until completion, coordinating via MCP Agent Mail tools.

## Quick Start

```bash
# 1. Ensure MCP server is running
am serve mcp --stdio  # For stdio transport
# OR
am serve http --port 8765 --with-ui &  # For HTTP/SSE transport

# 2. List available tools (47 total)
am tools
```

---

## MCP Tools Reference (Verified Working)

> **How to call**: Use your MCP client's tool calling mechanism with the JSON below.

### 1. Register Agent

```json
{
  "name": "register_agent",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "name": "BlueMountain",
    "program": "claude-code",
    "model": "claude-opus-4",
    "task_description": "Working on NTM-001"
  }
}
```

**Returns**: `{"id": 123, "name": "BlueMountain", "project_id": 122, ...}`

### 2. Send Message

```json
{
  "name": "send_message",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "sender_name": "BlueMountain",
    "to": "Coordinator",
    "subject": "[CLAIMING] NTM-001",
    "body_md": "Claiming this task.",
    "thread_id": "NTM-001",
    "importance": "normal",
    "ack_required": false
  }
}
```

**Note**: MCP tools use `to` (string, comma-separated for multiple). REST API uses `recipient_names` (array).

### 3. Reserve Files

```json
{
  "name": "file_reservation_paths",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "agent_name": "BlueMountain",
    "paths": [
      "crates/libs/lib-mcp/src/tools/mod.rs"
    ],
    "ttl_seconds": 3600,
    "exclusive": true,
    "reason": "NTM-001: Adding tool aliases"
  }
}
```

**Returns**: `{"granted": [...], "conflicts": []}`

### 4. Check Inbox

```json
{
  "name": "check_inbox",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "agent_name": "BlueMountain",
    "limit": 10
  }
}
```

**Returns**: Array of unread messages `[{"id": ..., "subject": ..., "sender_name": ...}, ...]`

### 5. Release Reservation

```json
{
  "name": "release_reservation",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "reservation_id": 22
  }
}
```

**Returns**: `{"released": true}`

### 6. List Agents

```json
{
  "name": "list_agents",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs"
  }
}
```

### 7. List Projects

```json
{
  "name": "list_projects",
  "arguments": {}
}
```

### MCP vs REST API Field Differences

| MCP Tool | MCP Field | REST API Field |
|----------|-----------|----------------|
| `send_message` | `to` (string) | `recipient_names` (array) |
| `send_message` | `cc` (string) | `cc_names` (array) |
| check inbox | `check_inbox` | `list_inbox` |
| release | `reservation_id` (int) | `paths` (array) |

---

## Git Workflow & Branch Strategy

### Branch Hierarchy

```
main (protected)
  │
  └── beads-sync (integration branch)
        │
        ├── feature/NTM-001 (agent worktree)
        ├── feature/NTM-002 (agent worktree)
        └── feature/SVELTE-001 (agent worktree)
```

### CRITICAL: Branch Rules

```
╔════════════════════════════════════════════════════════════════╗
║  🚨 AGENTS NEVER WORK ON MAIN BRANCH 🚨                        ║
║  ✅ Work ONLY on beads-sync or feature branches                ║
║  ✅ Use worktrees (.sandboxes/agent-<id>/)                     ║
║  ✅ Coordinator merges beads-sync → main                       ║
╚════════════════════════════════════════════════════════════════╝
```

### Three Isolation Layers

| Layer | Tool | Purpose |
|-------|------|---------|
| 1. File Reservations | MCP `file_reservation_paths` | Logical locks, prevent same-file conflicts |
| 2. Worktrees | `git worktree` | Physical isolation, no stash/pop needed |
| 3. beads-sync | Integration branch | `bd sync` commits here |

---

## Git Worktree Setup

### Creating Agent Worktree

```bash
# 1. Ensure beads-sync is up to date with main
git checkout beads-sync
git merge main --no-edit
git push origin beads-sync

# 2. Create isolated worktree for your task
git worktree add .sandboxes/agent-BlueMountain -b feature/NTM-001 beads-sync

# 3. Enter worktree
cd .sandboxes/agent-BlueMountain
```

### Worktree Directory Structure

```
mcp-agent-mail-rs/                 # Main repo (beads-sync branch)
├── .sandboxes/
│   ├── agent-BlueMountain/        # Worktree for BlueMountain (feature/NTM-001)
│   ├── agent-GreenCastle/         # Worktree for GreenCastle (feature/NTM-002)
│   └── agent-RedFalcon/           # Worktree for RedFalcon (feature/SVELTE-001)
└── ...
```

### Worktree Commands

```bash
# List all worktrees
git worktree list

# Create new worktree
git worktree add .sandboxes/agent-$ID -b feature/<task> beads-sync

# Remove worktree after completion
git worktree remove .sandboxes/agent-$ID

# Prune stale worktree refs
git worktree prune
```

### Completing Work in Worktree

```bash
# 1. In worktree: commit changes
git add -A
git commit -m "feat(mcp): add tool aliases for NTM-001"

# 2. Return to main repo
cd ../..

# 3. Merge feature into beads-sync
git checkout beads-sync
git merge feature/NTM-001 --no-edit
git push origin beads-sync

# 4. Delete feature branch
git branch -d feature/NTM-001

# 5. Remove worktree
git worktree remove .sandboxes/agent-BlueMountain
```

### Beads in Worktree

```bash
# IMPORTANT: Use --no-daemon flag in worktrees
bd --no-daemon ready
bd --no-daemon update <id> --status in_progress
bd --no-daemon close <id> --reason "Completed"
```

---

## Phase 1: Agent Registration

### Prerequisites
- MCP server running (stdio or HTTP transport)
- Project exists (use `list_projects` to check)

### Step 1.1: Ensure Project Exists

```json
{
  "name": "ensure_project",
  "arguments": {
    "slug": "mcp-agent-mail-rs",
    "human_key": "mcp-agent-mail-rs"
  }
}
```

### Step 1.2: Register Your Agent

```json
{
  "name": "register_agent",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "name": "BlueMountain",
    "program": "claude-code",
    "model": "claude-opus-4",
    "task_description": "Working on NTM-001"
  }
}
```

**Expected Response:**
```json
{"id": 123, "name": "BlueMountain", "project_id": 122, ...}
```

**Common Errors:**
| Error | Cause | Fix |
|-------|-------|-----|
| `Project not found` | Project doesn't exist | Run `ensure_project` first (Step 1.1) |
| `Agent already exists` | Name taken | Use different name or update existing |

**Naming Convention**: Use adjective+noun format (GreenCastle, RedFalcon, BlueMountain).

---

## Phase 2: Claim Work from Beads

### Step 2.1: Find Ready Work

```bash
bd ready --json
```

### Step 2.2: Ensure Coordinator Exists

**IMPORTANT**: Before sending messages to Coordinator, ensure it exists:

```json
{
  "name": "register_agent",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "name": "Coordinator",
    "program": "human",
    "model": "human",
    "task_description": "Project coordinator"
  }
}
```

### Step 2.3: Send Claiming Message

```json
{
  "name": "send_message",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "sender_name": "BlueMountain",
    "to": "Coordinator",
    "subject": "[CLAIMING] NTM-001: Add tool aliases",
    "body_md": "Claiming issue mcp-agent-mail-rs-46xk",
    "thread_id": "NTM-001",
    "importance": "normal"
  }
}
```

**Note**: MCP uses `to` (comma-separated string). For multiple recipients: `"to": "Coordinator,Reviewer"`.

**Common Errors:**
| Error | Cause | Fix |
|-------|-------|-----|
| `Agent not found: Coordinator` | Recipient doesn't exist | Register Coordinator first (Step 2.2) |

### Step 2.4: Reserve Files (BEFORE editing)

```json
{
  "name": "file_reservation_paths",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "agent_name": "BlueMountain",
    "paths": ["crates/libs/lib-mcp/src/tools/mod.rs"],
    "ttl_seconds": 3600,
    "exclusive": true,
    "reason": "NTM-001: Adding tool aliases"
  }
}
```

**Expected Response:**
```json
{"granted": [{"id": 22, "path_pattern": "...", "expires_ts": "..."}], "conflicts": []}
```

**If conflicts not empty**: Another agent has the files. Check their inbox and coordinate.

---

## Phase 3: Autonomous Work Loop

### The Autonomous Agent Loop

```
┌──────────────────────────────────────────────────────────────┐
│                    AUTONOMOUS WORK LOOP                       │
│                                                              │
│  ┌─────────────┐                                             │
│  │ check_inbox │◄───────────────────────────────────┐        │
│  └──────┬──────┘                                    │        │
│         │ New messages?                              │        │
│         ▼                                            │        │
│  ┌──────────────┐    Yes    ┌──────────────────┐    │        │
│  │ Process msg  │◄─────────│ Requires action? │    │        │
│  └──────┬───────┘           └────────┬─────────┘    │        │
│         │                             │ No          │        │
│         ▼                             ▼             │        │
│  ┌──────────────┐           ┌─────────────────┐    │        │
│  │   Do Work    │           │ mark_message_   │    │        │
│  │ (edit code)  │           │     read        │    │        │
│  └──────┬───────┘           └────────┬────────┘    │        │
│         │                            │             │        │
│         ▼                            │             │        │
│  ┌──────────────┐                    │             │        │
│  │ send_message │ ───────────────────┘             │        │
│  │ (progress)   │                                  │        │
│  └──────┬───────┘                                  │        │
│         │                                          │        │
│         ▼                                          │        │
│  ┌──────────────┐    No     ┌───────────────┐      │        │
│  │ Task done?   │──────────►│ renew_file_   │──────┘        │
│  └──────┬───────┘           │ reservation   │               │
│         │ Yes               └───────────────┘               │
│         ▼                                                   │
│  ┌──────────────┐                                           │
│  │ bd close     │                                           │
│  └──────┬───────┘                                           │
│         │                                                   │
│         ▼                                                   │
│  ┌────────────────────┐                                     │
│  │ release_reservation│                                     │
│  └────────────────────┘                                     │
└──────────────────────────────────────────────────────────────┘
```

### Step 3.1: Check Inbox Regularly

```json
{
  "name": "check_inbox",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "agent_name": "BlueMountain",
    "limit": 10
  }
}
```

**Returns**: Array of unread messages `[{"id":..., "subject":..., "sender_name":...}, ...]`

### Step 3.2: Process Messages

**For blocking messages (ack_required=true)**:
```json
{
  "name": "acknowledge_message",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "message_id": 123,
    "agent_name": "BlueMountain"
  }
}
```

**For info-only messages**:
```json
{
  "name": "mark_message_read",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "message_id": 123,
    "agent_name": "BlueMountain"
  }
}
```

### Step 3.3: Report Progress

```json
{
  "name": "send_message",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "sender_name": "BlueMountain",
    "to": "Coordinator",
    "subject": "[PROGRESS] NTM-001: 50% complete",
    "body_md": "## Progress Update\n\n- [x] Added alias mapping in call_tool\n- [ ] Sync schema\n- [ ] Add tests\n\n**Blockers**: None\n**ETA**: Next commit",
    "thread_id": "NTM-001"
  }
}
```

### Step 3.4: Renew Reservations (if work takes longer)

```json
{
  "name": "renew_file_reservation",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "reservation_id": 42,
    "ttl_seconds": 3600
  }
}
```

### Step 3.5: Request Help from Other Agents

```json
{
  "name": "send_message",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "sender_name": "BlueMountain",
    "to": "GreenCastle,RedFalcon",
    "subject": "[HELP] Need review on NTM-001",
    "body_md": "Can someone review my changes to `call_tool` alias mapping?\n\n```rust\nlet tool_name = match request.name.as_str() {\n    \"fetch_inbox\" => \"list_inbox\",\n    other => other,\n}.to_string();\n```",
    "thread_id": "NTM-001",
    "ack_required": true
  }
}
```

---

## Phase 4: Coordination Patterns

### Pattern 1: Handoff to Another Agent

When blocked or need specialized help:

```json
{
  "name": "send_message",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "sender_name": "BlueMountain",
    "to": "GreenCastle",
    "subject": "[HANDOFF] NTM-003 requires lib-core changes",
    "body_md": "## Handoff Request\n\n**Task**: NTM-003 (create_agent_identity)\n**Reason**: Requires changes to lib-core name generation\n**Files needed**: `crates/libs/lib-core/src/model/agent.rs`\n\nI will release my reservation on lib-mcp files. Please claim lib-core.",
    "thread_id": "NTM-003",
    "importance": "high",
    "ack_required": true
  }
}
```

### Pattern 2: Parallel Work Notification

When discovering work that can be parallelized:

```json
{
  "name": "send_message",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "sender_name": "BlueMountain",
    "to": "Coordinator",
    "subject": "[PARALLEL] Found independent subtasks",
    "body_md": "## Parallel Work Available\n\nWhile working on NTM-001, I identified these can run in parallel:\n\n1. **NTM-002** (list_project_agents) - no file conflicts with NTM-001\n2. **NTM-004** (macro_start_session) - separate file set\n\nSuggestion: Spawn additional agents for throughput.",
    "thread_id": "NTM-COORDINATION"
  }
}
```

### Pattern 3: Conflict Resolution

When file reservation conflicts occur:

```json
{
  "name": "send_message",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "sender_name": "BlueMountain",
    "to": "RedFalcon",
    "subject": "[CONFLICT] File reservation overlap",
    "body_md": "## Conflict Detected\n\n**File**: `crates/libs/lib-mcp/src/tools/mod.rs`\n**My task**: NTM-001 (aliases)\n**Your task**: ?\n\n**Proposal**:\n1. I take lines 1700-1800 (call_tool function)\n2. You take other sections\n\nOr: I finish first (~30 min), then release.",
    "importance": "urgent",
    "ack_required": true
  }
}
```

---

## Phase 5: Task Completion

### Step 5.1: Close Bead

```bash
bd close mcp-agent-mail-rs-46xk --reason "Implemented alias mapping, schema sync, and tests"
```

### Step 5.2: Release Reservations

```json
{
  "name": "release_reservation",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "reservation_id": 22
  }
}
```

**Note**: MCP uses `reservation_id` (from the grant response). REST API uses `paths` array.

**Expected Response:**
```json
{"released": true}
```

### Step 5.3: Send Completion Notification

```json
{
  "name": "send_message",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "sender_name": "BlueMountain",
    "to": "Coordinator",
    "subject": "[COMPLETION] NTM-001: Tool aliases",
    "body_md": "Task completed. See commit abc1234.",
    "thread_id": "NTM-001",
    "importance": "high",
    "ack_required": true
  }
}
```

### Step 5.4: Check for More Work

```bash
bd ready --json
```

If more work available, return to Phase 2.

---

## MCP Agent Mail Tools Reference (47 total)

### Core Messaging
| Tool | Description |
|------|-------------|
| `send_message` | Send message to agents (to, cc, bcc) |
| `reply_message` | Reply to existing thread |
| `check_inbox` | Get unread messages |
| `list_outbox` | Sent messages |
| `get_message` | Get message by ID |
| `mark_message_read` | Mark as read |
| `acknowledge_message` | Mark as acknowledged |

### Search & Threads
| Tool | Description |
|------|-------------|
| `search_messages` | Full-text search in project |
| `search_messages_product` | Search across product |
| `list_threads` | List conversation threads |
| `summarize_thread` | AI summary of thread |
| `summarize_thread_product` | Summary across product |
| `list_pending_reviews` | Messages awaiting ack |

### File Reservations
| Tool | Description |
|------|-------------|
| `file_reservation_paths` | Reserve multiple paths |
| `reserve_file` | Reserve single file |
| `release_reservation` | Release by ID |
| `list_file_reservations` | List active reservations |
| `renew_file_reservation` | Extend TTL |
| `force_release_reservation` | Emergency override |

### Agent Identity
| Tool | Description |
|------|-------------|
| `register_agent` | Register/update agent |
| `list_agents` | List project agents |
| `get_agent_profile` | Detailed agent info |
| `whois` | Agent lookup (alias) |

### Project Management
| Tool | Description |
|------|-------------|
| `ensure_project` | Create/get project |
| `list_projects` | All projects |
| `get_project_info` | Project details |

### Contacts & Policies
| Tool | Description |
|------|-------------|
| `request_contact` | Request contact permission |
| `respond_contact` | Accept/reject contact |
| `list_contacts` | Agent's contacts |
| `set_contact_policy` | open/auto/contacts_only/block_all |

### Build Slots (CI/CD)
| Tool | Description |
|------|-------------|
| `acquire_build_slot` | Exclusive build access |
| `release_build_slot` | Release slot |
| `renew_build_slot` | Extend slot TTL |

### Macros
| Tool | Description |
|------|-------------|
| `list_macros` | Available macros |
| `register_macro` | Define new macro |
| `invoke_macro` | Execute macro |

### Products (Multi-Repo)
| Tool | Description |
|------|-------------|
| `ensure_product` | Create product |
| `link_project_to_product` | Link project |
| `list_products` | All products |
| `product_inbox` | Aggregated inbox |

### Utilities
| Tool | Description |
|------|-------------|
| `export_mailbox` | Export to HTML/JSON/MD |
| `add_attachment` | Attach file to message |
| `get_attachment` | Retrieve attachment |
| `install_precommit_guard` | Install git hook |
| `uninstall_precommit_guard` | Remove git hook |
| `list_tool_metrics` | Usage metrics |
| `get_tool_stats` | Aggregated stats |
| `list_activity` | Project activity log |

---

## Beads Integration

### Essential Commands

```bash
# Find unblocked work
bd ready --json

# Claim work
bd update <id> --status in_progress

# Create discovered issues
bd create "Found bug" --description="Details" -t bug --deps discovered-from:<id>

# Complete work
bd close <id> --reason "Completed"

# Sync at session end
bd sync
```

### Status Flow

```
open → in_progress → [completed | blocked]
```

### Priority Levels

| Priority | Meaning |
|----------|---------|
| P0 | Critical (security, data loss) |
| P1 | High (major features, important bugs) |
| P2 | Medium (nice-to-have) |
| P3 | Low (polish) |
| P4 | Backlog |

---

## Autonomous Operation Rules

### MUST DO
- [x] Register identity before any work
- [x] Reserve files before editing
- [x] Check inbox regularly (every major step)
- [x] Acknowledge blocking messages
- [x] Report progress via send_message
- [x] Release reservations when done
- [x] Close beads when complete
- [x] Run `bd sync` at session end

### MUST NOT
- [ ] Edit files without reservation
- [ ] Ignore messages with `ack_required=true`
- [ ] Work on main branch directly
- [ ] Force push to shared branches
- [ ] Skip the completion notification

### SHOULD DO
- Renew reservations before expiry
- Notify on blockers immediately
- Request handoff when stuck
- Create follow-on beads for discovered work
- Summarize long threads for context

---

## Multi-Agent Orchestration

### Agent Roles

| Role | Name Pattern | Responsibility |
|------|--------------|----------------|
| Worker | `worker-<id>` or adjective+noun | Implements task, runs quality gates |
| Reviewer | `reviewer` | Validates work, fixes issues |
| Human | `human` | Final oversight, merges to main |
| Coordinator | `Coordinator` | Assigns work, manages conflicts |

### Workflow State Machine

```
BEADS → WORKER → [COMPLETION] mail → REVIEWER → [APPROVED/FIXED] → HUMAN
        (exits)    (async)           (picks up)
```

**Key**: Worker sends [COMPLETION] and **EXITS**. Does NOT wait for [APPROVED].

### Subject Prefix Protocol

```
[CLAIMING]     → Agent claiming a task
[TASK_STARTED] → Work beginning (optional)
[PROGRESS]     → Progress update
[HELP]         → Request assistance
[HANDOFF]      → Transfer to another agent
[CONFLICT]     → File reservation conflict
[COMPLETION]   → Task done, ready for review
[REVIEWING]    → Reviewer claiming review
[APPROVED]     → Review passed
[REJECTED]     → Review failed, needs rework
[FIXED]        → Reviewer fixed issues
[ACK]          → Human acknowledgment
[URGENT]       → Priority escalation
[PARALLEL]     → Parallel work opportunity
```

### Worker Phase

1. `bd ready` → `bd update <id> --status in_progress`
2. `register_agent` → `file_reservation_paths`
3. Create worktree, implement, run quality gates
4. Commit, merge to beads-sync
5. Send `[COMPLETION]` mail (to=reviewer, cc=human, ack_required=true)
6. Release reservations, **EXIT**

### Reviewer Phase

1. `check_inbox` for `[COMPLETION]` mails
2. Check thread state (skip if `[APPROVED]` or `[REVIEWING]` exists)
3. Send `[REVIEWING]` to claim
4. **Validate**: Read files, check placeholders, verify acceptance criteria, run gates
5. If PASS: Send `[APPROVED]`, close task
6. If FAIL: Fix in worktree, send `[FIXED]`, close task

### CC Rules

| Message | To | CC |
|---------|----|----|
| Worker [COMPLETION] | reviewer | human |
| Reviewer [REVIEWING/APPROVED/FIXED] | worker | human |
| Human [ACK] | reviewer | worker |

### Single-Agent Fallback

If no reviewer available: Worker self-reviews and sends `[COMPLETION] (Self-Reviewed)` directly to Human.

---

## Quality Gates

### Required Before Commit

```bash
# Run ALL in worktree before committing
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test -p lib-core --test integration -- --test-threads=1
```

### Validation Checklist

- [ ] Zero `todo!()`, `unimplemented!()` in src/
- [ ] 100% acceptance criteria mapped to code
- [ ] All quality gates pass
- [ ] No OWASP vulnerabilities
- [ ] Conventional commit message format

### Build Slots (CI/CD Coordination)

When running builds that need exclusive access:

```json
{
  "name": "acquire_build_slot",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "agent_name": "BlueMountain",
    "slot_type": "ci",
    "ttl_seconds": 600,
    "reason": "Running integration tests"
  }
}
```

Release after build:

```json
{
  "name": "release_build_slot",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "slot_id": 1
  }
}
```

---

## Message Templates

### [COMPLETION] (Worker → Reviewer)

```markdown
## Task Completion Report

**Task ID**: mcp-agent-mail-rs-46xk | **Commit**: abc1234

### Files Changed
- crates/libs/lib-mcp/src/tools/mod.rs

### Acceptance Criteria
- [x] Added alias mapping in call_tool
- [x] Synced schema with implementations
- [x] Added unit tests

### Quality Gates
✅ check ✅ clippy ✅ fmt ✅ test
```

### [APPROVED] (Reviewer → Worker, cc Human)

```markdown
## Review Complete - APPROVED

Implementation complete, criteria met, gates passed.
Merged to beads-sync at commit def5678.
```

### [FIXED] (Reviewer → Worker, cc Human)

```markdown
## Review Complete - FIXED

Found minor issues, fixed in commit ghi9012:
- Fixed missing error handling in line 234
- Added missing test case

Task closed.
```

---

## Emergency Procedures

### Force Release (stuck reservation)
```json
{
  "name": "force_release_reservation",
  "arguments": {
    "project_slug": "mcp-agent-mail-rs",
    "reservation_id": 42,
    "reason": "Agent unresponsive for >1 hour"
  }
}
```

### Urgent Message
```json
{
  "name": "send_message",
  "arguments": {
    "sender_name": "BlueMountain",
    "to": "Coordinator",
    "subject": "[URGENT] Build broken",
    "importance": "urgent",
    "ack_required": true
  }
}
```

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `bd` shows "database not found" | `bd init --quiet` |
| `bd` in worktree fails | Use `bd --no-daemon` flag |
| Test DB conflicts | Add `--test-threads=1` |
| File reservation conflict | Check inbox, coordinate with holder |
| Worktree already exists | `git worktree remove <path>` first |
| Branch already used by worktree | `git worktree prune` |
| Merge conflicts on beads-sync | Pull latest, resolve, push |
| Build slot unavailable | Check `list_build_slots`, wait or request |

---

## Session End Checklist

```bash
# Complete ALL before exiting
[ ] 1. All quality gates pass (check, clippy, fmt, test)
[ ] 2. Changes committed in worktree
[ ] 3. Feature branch merged to beads-sync
[ ] 4. Worktree removed: git worktree remove .sandboxes/agent-$ID
[ ] 5. All file reservations released via MCP
[ ] 6. [COMPLETION] message sent to reviewer
[ ] 7. All active beads closed or updated with notes
[ ] 8. bd sync completed
[ ] 9. git push origin beads-sync
[ ] 10. git status shows clean working tree
```

### Worktree Cleanup Script

```bash
#!/bin/bash
# cleanup-agent.sh <agent-name> <task-id>
AGENT=$1
TASK=$2

# In worktree: final commit
cd .sandboxes/agent-$AGENT
git add -A && git commit -m "feat: complete $TASK" --allow-empty

# Back to main repo
cd ../..
git checkout beads-sync
git merge feature/$TASK --no-edit
git push origin beads-sync

# Cleanup
git branch -d feature/$TASK
git worktree remove .sandboxes/agent-$AGENT
```

---

## Full Agent Lifecycle Example

```bash
# === STARTUP ===
git checkout beads-sync && git pull origin beads-sync
git merge main --no-edit

# Register (via MCP)
# ensure_project(slug="/path/to/repo", human_key="mcp-agent-mail-rs")
# register_agent(project_slug="mcp-agent-mail-rs", name="BlueMountain", ...)

# Find work
bd ready --json

# Claim task (via MCP)
# send_message(to="Coordinator", subject="[CLAIMING] NTM-001", ...)

# Reserve files (via MCP - BEFORE creating worktree)
# file_reservation_paths(paths=["crates/libs/lib-mcp/src/tools/*.rs"], ...)

# Create worktree
git worktree add .sandboxes/agent-BlueMountain -b feature/NTM-001 beads-sync
cd .sandboxes/agent-BlueMountain
bd --no-daemon update mcp-agent-mail-rs-46xk --status in_progress

# === WORK LOOP ===
# Check inbox regularly (via MCP)
# check_inbox(project_slug="mcp-agent-mail-rs", agent_name="BlueMountain")

# Do work...
# Edit files, implement feature

# Run quality gates
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test -p lib-core --test integration -- --test-threads=1

# Commit
git add -A
git commit -m "feat(mcp): add tool aliases for NTM compatibility"

# === COMPLETION ===
# Return to main repo
cd ../..

# Merge to beads-sync
git checkout beads-sync
git merge feature/NTM-001 --no-edit
git push origin beads-sync

# Release reservations (via MCP)
# release_reservation(project_slug="mcp-agent-mail-rs", reservation_id=42)

# Close bead
bd close mcp-agent-mail-rs-46xk --reason "Completed: alias mapping, schema sync, tests"
bd sync

# Send completion (via MCP)
# send_message(to="reviewer", cc="human", subject="[COMPLETION] NTM-001", ack_required=true, ...)

# Cleanup worktree
git branch -d feature/NTM-001
git worktree remove .sandboxes/agent-BlueMountain

# Check for more work
bd ready --json
# If more work: loop back to STARTUP
# If no work: EXIT
```

---

**Next agent prompt**:
```
Continue work on mcp-agent-mail-rs. Run `bd ready --json` to find unblocked tasks.
Check inbox: check_inbox(project_slug="mcp-agent-mail-rs", agent_name="<your-name>")
If starting fresh: Register first, then create worktree.
```

---

**Version**: 1.0.0 | **Last Updated**: 2025-12-22
