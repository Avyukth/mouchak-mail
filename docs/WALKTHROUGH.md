# Integration Scripts (dlf) - Walkthrough

## Summary

Created 10 shell scripts to configure MCP servers for all major coding agents.

## Scripts Created

| Script | Agent | Config Path |
|--------|-------|-------------|
| [integrate_claude_code.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_claude_code.sh) | Claude Code | `~/.claude.json` |
| [integrate_cursor.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_cursor.sh) | Cursor IDE | `~/.cursor/mcp.json` |
| [integrate_windsurf.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_windsurf.sh) | Windsurf | `~/.codeium/windsurf/mcp_config.json` |
| [integrate_cline.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_cline.sh) | Cline | VSCode global storage |
| [integrate_codex_cli.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_codex_cli.sh) | Codex CLI | `~/.codex/config.json` |
| [integrate_gemini_cli.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_gemini_cli.sh) | Gemini CLI | `~/.gemini/settings.json` |
| [integrate_github_copilot.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_github_copilot.sh) | GitHub Copilot | `~/.vscode/mcp.json` |
| [integrate_opencode.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_opencode.sh) | OpenCode | `~/.opencode/config.json` |
| [integrate_antigravity.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_antigravity.sh) | Antigravity | `~/.gemini/mcp_servers.json` |
| [integrate_all.sh](file:///Users/amrit/Documents/Projects/Rust/mouchak/mcp-agent-mail-rs/scripts/integrate_all.sh) | Auto-detect | All applicable |

## Usage

**Auto-detect and configure all:**
```bash
./scripts/integrate_all.sh
```

**Detection only (preview):**
```bash
./scripts/integrate_all.sh --detect-only
```

**Individual agent:**
```bash
./scripts/integrate_claude_code.sh --mode stdio
./scripts/integrate_cursor.sh --scope project
```

## Verification

Ran `integrate_all.sh --detect-only`:
```
✓ Claude Code detected
✓ Cursor IDE detected
○ Windsurf IDE not found
○ Cline (VSCode) not found
✓ Codex CLI detected
✓ Gemini CLI detected
✓ GitHub Copilot detected
○ OpenCode not found
✓ Antigravity detected

✓ Detected 6 coding agent(s)
```

## Features

- **jq-based JSON merge** - Preserves existing config
- **Backup creation** - Creates timestamped backups
- **Verification** - Checks if server is running
- **HTTP/STDIO modes** - Claude Code supports both

## MCP Resources Implementation (if9)

### Summary
Implemented standard MCP resource handlers (`list_resources`, `read_resource`) in `mcp-stdio` service, enabling agents to browse:
- Project agents (`agent-mail://{slug}/agents`)
- File reservations (`agent-mail://{slug}/file_reservations`)
- Inboxes (`agent-mail://{slug}/inbox/{agent}`)
- Outboxes (`agent-mail://{slug}/outbox/{agent}`)
- Threads (`agent-mail://{slug}/thread/{id}`)

### Antigravity Integration
- Updated UI placeholders to suggest `antigravity` and `gemini-2.0-pro`.
- Validated `antigravity` program name in integration tests.
- Fixed `MessageForCreate` struct field initialization (`cc_ids`, `bcc_ids`) across test suite.

### Verification
- `cargo check -p mcp-stdio` passing.
- `cargo test -p lib-core -p mcp-stdio` passing.



## Capabilities & RBAC Middleware

### Goal
Restrict sensitive agent actions based on assigned capabilities.

### Changes
- **Schema**: Added `agent_capabilities` table (migration `002`).
### Project Siblings Endpoints (y58)
- Added `list_for_project` to `ProductBmc` to find products a project belongs to.
- Added `list_siblings` to `ProjectBmc` to find related projects via shared products.
- Implemented `list_project_siblings` tool handling the logic and formatting the output.

### Git Archive Integration (azc)
- Implemented `ProjectBmc::sync_to_archive` to dumping project mailbox and agent data to JSON files in the repo.
- Used `git_store` to commit these files with a specified message.
- Exposed functionality via `commit_archive` tool.
- Verified end-to-end with `test_git_archive_workflow` integration test.

# Master Video Walkthrough Script
> **Purpose**: A step-by-step screenplay for recording the official demo video of the Agent Mail application.
> **Environment**: Mobile Viewport (iPhone 14 Pro/SE).

## 1. Scene: Dashboard & Aesthetics
**Action**: Start recording.
1.  **Opening Shot**: Dashboard is visible in **Light Mode**.
2.  **Theme Toggle**:
    -   Locate the **Moon Icon** (top-right).
    -   Tap it. Observe the transition to **Dark Mode**.
    -   *Commentary*: "Seamlessly switch between light and dark themes."
3.  **Recent Activity**:
    -   Scroll down the dashboard to show the "Recent Activity" feed.
    -   *Commentary*: "Stay updated with the latest agent communications."

## 2. Scene: Project Management
1.  **Navigation**: Tap **Projects** (bottom bar).
2.  **View List**: Scroll through existing projects (Cards).
3.  **Create Project**:
    -   Tap **"New Project"**.
    -   Input Path: `/tmp/demo-project` (or similar).
    -   Tap **"Create Project"**.
    -   *Verify*: Project appears in the list.

## 3. Scene: Agent Directory
1.  **Navigation**: Tap **"View Agents"** on any project card OR tap `Agents` in navigation (if present, otherwise assume implicit).
    -   *Correction*: Tap the explicit **Agents** nav item if available, or just browse the global list. (Code shows global Agents page).
2.  **Search**:
    -   Type "Alice" in the search bar.
    -   Verify list filters.

## 4. Scene: Inbox & Communication (The Core Feature)
1.  **Navigation**: Tap **Inbox**.
2.  **Select Context**:
    -   Select Project: **"Demo Project"**.
    -   Select Agent: **"Alice"**.
3.  **Scenario A: Sending a High-Priority Message**:
    -   Tap **Compose** (Pen Icon).
    -   **Subject**: "System Alert".
    -   **To**: Select "Bob".
    -   **Importance**: Change to **High** (Red).
    -   **Ack Required**: Check the box.
    -   **Body**: "Main server needs reboot."
    -   Tap **Send Message**.
    -   *Visual*: Verify the message appears in the inbox list with a Red High-Priority badge.
4.  **Scenario B: Cancelling a Draft**:
    -   Tap **Compose** again.
    -   Type "Mistake text".
    -   Tap **Cancel**.
    -   *Visual*: Modal closes, no message sent.
5.  **Threaded Reply**:
    -   Tap the message you just sent ("System Alert").
    -   Tap **Reply**.
    -   **Body**: "Reboot initiated."
    -   Tap **Send Message**.
    -   *Visual*: Message sent.

## 5. Scene: Conclusion
1.  **Navigation**: Return to **Dashboard**.
2.  **Closing**: Show the updated activity feed reflecting the new messages.
**Action**: Stop recording.

### [f51] Built-in Macros Registration Test
- **Implementation**: Created `crates/libs/lib-core/tests/macro_tests.rs` to verify that built-in macros (e.g., `start_session`, `prepare_thread`) are automatically registered when a project is created.
- **Verification**: Ran `cargo test --test macro_tests` and confirmed 5 built-in macros are present and logic is idempotent.

### [tlm] Tool Metrics API
- **Implementation**:
    - Migration `003` for `tool_metrics` table.
    - `ToolMetric` model and BMC in `lib-core` using raw `libsql`.
    - `mcp-stdio` instrumented to record metrics (duration, args, status) async.
    - `mcp-server` endpoints (`/api/metrics/tools`) exposed.
- **Verification**:
    - Added `test_record_tool_metric` in `mcp-stdio/src/tools.rs`.
    - Verified metrics are inserted with correct context (project/agent).
    - Verified `libsql` integration with parameter binding.

### [rct] Recent Activity API
- **Implementation**:
    - Created `ActivityBmc` in `lib-core` merging `messages` (created_ts), `tool_metrics` (created_at), `agents` (inception_ts).
    - Upgraded `tool_metrics` to record microsecond-precision timestamps to ensure correct sort order vs messages.
    - Added `GET /api/activity` endpoint in `mcp-server`.
- **Verification**:
    - Created `crates/services/mcp-stdio/tests/activity_tests.rs`.
    - Verified that tool usages, messages, and agent creations are correctly interleaved and sorted by time in descending order.

### [sys] Systemd Service Files
- **Implementation**:
    - Created `deploy/systemd/mcp-server.service`: Standard systemd unit file for running `mcp-server`.
    - Created `deploy/systemd/mcp-server.env.example`: Template for production environment variables.
    - Created `scripts/install_service.sh`: Shell script to automate installation (create user, copy files, reload daemon) on Linux.
- **Verification**:
    - Verified script enforces root execution (ran without root, confirmed exit code 1).
    - Code review of unit file for standard best practices (Restart=always, User=mcp).

---

## Automated Video Walkthrough

### Overview
Fully automated browser walkthrough that records a demo video using Playwright for browser automation.

### Prerequisites
```bash
# Install dependencies
cd scripts/video-walkthrough
bun install
```

### Running the Walkthrough

1. **Start the backend and frontend**:
```bash
# Terminal 1: Backend
cargo run -p mcp-server

# Terminal 2: Frontend (SvelteKit)
cd crates/services/web-ui && bun run dev
```

2. **Run the automated walkthrough**:
```bash
cd scripts/video-walkthrough

# Without recording (browser visible)
bun run walkthrough.ts

# With video recording
bun run walkthrough.ts --record
```

3. **Output**: Video saved as `walkthrough_<timestamp>.webm`

### What Gets Recorded
The walkthrough covers 8 scenes:
1. **Dashboard & Theme** - Opens dashboard, toggles dark mode
2. **Projects** - Lists projects, creates new project `/demo/video-walkthrough`
3. **Agents** - Registers AlphaBot (claude-code) and BetaBot (gemini)
4. **All Agents** - Navigates to agents list, demonstrates search
5. **Inbox & Compose** - Selects project/agent, composes high-priority message with acknowledgment
6. **Message Detail & Reply** - Views message as recipient, sends reply
7. **Mobile View** - Resizes to 375×812 viewport, navigates key screens
8. **Wrap Up** - Returns to desktop, toggles light mode, final dashboard shot

### Timing Configuration
Located in `scripts/video-walkthrough/walkthrough.ts`:
```typescript
const DELAY = {
  SHORT: 500,      // Quick pauses
  ACTION: 1200,    // Between user actions
  SCENE: 2000,     // Between major scenes
  TYPE: 50,        // Per character typing
};
```

### Narration Scripts
For generating audio narration:
- `docs/WALKTHROUGH_NARRATION.srt` - SRT subtitle file with timestamps
- `docs/WALKTHROUGH_NARRATION_SCRIPT.md` - TTS-optimized script with sync reference
