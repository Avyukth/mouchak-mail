# MCP Agent Mail Integration Scripts

This directory contains integration scripts for various AI coding agents and IDEs to work with mcp-agent-mail.

## Available Integration Scripts

### 1. claude-code.sh - Claude Code CLI
**Target**: Claude Code command-line interface
**Config Locations**:
  - User config: `~/.claude.json`
  - Project config (shared): `.mcp.json`
**Transport**: STDIO (default) or SSE

Configures Claude Code CLI to use MCP Agent Mail for agent communication.

```bash
./claude-code.sh                     # User scope, STDIO mode
./claude-code.sh --scope project     # Project scope (.mcp.json)
./claude-code.sh --mode sse          # SSE mode instead of STDIO
```

**Features**:
- Auto-detects Claude Code installation
- Supports user-scope (`~/.claude.json`) and project-scope (`.mcp.json`) configs
- STDIO transport (default) or SSE transport
- Creates backup of existing settings
- Verifies installation

---

### 2. cursor.sh - Cursor IDE
**Target**: Cursor IDE
**Config Location**: `~/.cursor/mcp_settings.json`
**Transport**: STDIO

Sets up Cursor IDE's Composer to use MCP Agent Mail tools.

```bash
./cursor.sh
```

**Features**:
- Detects Cursor installation
- Configures MCP server for Cursor's AI features
- Tools available in Cmd+I/Ctrl+I composer
- Supports multi-agent coordination

---

### 3. cline.sh - Cline VSCode Extension
**Target**: Cline (formerly Claude Dev) extension for VSCode
**Config Location**: Platform-specific VSCode global storage
- macOS: `~/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json`
- Linux: `~/.config/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json`

**Transport**: STDIO

Integrates MCP Agent Mail with the Cline VSCode extension.

```bash
./cline.sh
```

**Features**:
- Checks for VSCode and Cline extension
- Platform-aware configuration
- Enables agent coordination within VSCode
- Message-based collaboration

---

### 4. windsurf.sh - Windsurf IDE
**Target**: Windsurf IDE (Codeium)
**Config Location**: `~/.codeium/windsurf/mcp_config.json`
**Transport**: STDIO

Configures Windsurf's Cascade AI to use MCP Agent Mail.

```bash
./windsurf.sh
```

**Features**:
- Detects Windsurf installation
- Integrates with Cascade AI panel
- Supports file reservations
- Multi-agent refactoring coordination

---

### 5. aider.sh - Aider
**Target**: Aider command-line AI coding assistant
**Config Location**: `~/.aider.conf.yml`
**Transport**: STDIO wrapper

Creates a wrapper script for Aider since it doesn't have native MCP support.

```bash
./aider.sh
```

**Features**:
- Creates `aider-with-mail` wrapper script
- Starts MCP server alongside Aider
- Adds configuration guidance
- Process management for server lifecycle

**Usage After Install**:
```bash
aider-with-mail
```

---

### 6. continue.sh - Continue.dev
**Target**: Continue.dev VSCode extension
**Config Location**: `~/.continue/config.json`
**Transport**: STDIO

Configures Continue.dev's experimental MCP support.

```bash
./continue.sh
```

**Features**:
- Uses Continue's experimental MCP features
- Adds to `experimental.modelContextProtocolServers`
- Context-aware agent registration
- Integration with Continue's Cmd+L/Ctrl+L interface

**Note**: MCP support in Continue.dev is experimental.

---

### 7. copilot.sh - GitHub Copilot
**Target**: GitHub Copilot (VSCode/JetBrains)
**Config Location**: `.vscode/settings.json` (workspace)
**Transport**: HTTP REST API

Creates workspace configuration for Copilot to access MCP Agent Mail via HTTP API.

```bash
./copilot.sh
```

**Features**:
- Creates workspace-level VSCode settings
- Generates `.copilot-instructions.md` with API guidance
- Creates `start-mcp-mail.sh` server script
- Uses HTTP API (Copilot doesn't support MCP natively)

**Usage After Install**:
```bash
./start-mcp-mail.sh  # In terminal
# Then use Copilot Chat with API instructions
```

---

### 8. generic-mcp.sh - Generic MCP Client
**Target**: Any MCP-compatible client
**Output**: `mcp-agent-mail-configs/` directory
**Transport**: All (STDIO, HTTP, SSE)

Generates example configurations for any MCP client.

```bash
./generic-mcp.sh
```

**Output Files**:
- `stdio-config.json` - STDIO transport configuration
- `http-config.json` - HTTP REST API configuration
- `sse-config.json` - Server-Sent Events configuration
- `README.md` - Detailed usage instructions

**Features**:
- Creates configs for all transport modes
- Comprehensive documentation
- Copy-paste ready configurations
- Environment variable documentation

---

### 9. opencode.sh - OpenCode
**Target**: OpenCode AI coding assistant
**Config Locations**:
  - Project config: `.opencode/mcp.json`
  - Global config: `~/.config/opencode/config.json`
**Transport**: STDIO

Configures OpenCode to use MCP Agent Mail for agent coordination.

```bash
./opencode.sh                    # Project config (.opencode/mcp.json)
./opencode.sh --global           # Global config (~/.config/opencode/)
./opencode.sh --project /path    # Specific project directory
```

**Features**:
- Auto-detects OpenCode installation
- Supports project-scope and global configs
- STDIO transport for efficient communication
- Creates backup of existing settings

---

### 10. antigravity.sh - Antigravity
**Target**: Antigravity AI coding agent
**Config Location**: `~/.antigravity/mcp.json`
**Transport**: STDIO

Configures Antigravity to use MCP Agent Mail.

```bash
./antigravity.sh
```

**Features**:
- Detects Antigravity installation (including `ag` alias)
- Configures STDIO-based MCP server
- Creates backup of existing config
- Verifies installation

---

### 11. gemini.sh - Gemini CLI
**Target**: Google Gemini CLI
**Config Location**: `~/.gemini/settings/mcp_settings.json`
**Transport**: HTTP

Configures Gemini CLI to use MCP Agent Mail via HTTP transport.

```bash
./gemini.sh
```

**Features**:
- Detects Gemini CLI installation
- Uses HTTP transport for Gemini's MCP support
- Auto-creates config directory if needed
- Creates backup of existing settings

**Note**: Requires MCP Agent Mail HTTP server to be running:
```bash
am serve http --port 8765
```

---

### 12. codex.sh - OpenAI Codex CLI
**Target**: OpenAI Codex CLI
**Config Location**: `.codex/config.toml` (project-local)
**Transport**: HTTP

Configures OpenAI Codex CLI to use MCP Agent Mail.

```bash
./codex.sh                       # Configure in current directory
./codex.sh --project /path       # Configure in specific project
```

**Features**:
- Uses TOML config format (Codex-specific)
- Project-local configuration
- HTTP transport for MCP communication
- Creates backup of existing config

**Note**: Requires MCP Agent Mail HTTP server to be running:
```bash
am serve http --port 8765
```

---

## Common Usage Pattern

All integration scripts follow this pattern:

1. **Check dependencies** (jq, target client)
2. **Detect client installation**
3. **Locate MCP server binary** (mcp-stdio-server or mcp-server)
4. **Update/create configuration file**
5. **Verify installation**
6. **Print summary with next steps**

## Prerequisites

All scripts require:
- **jq** - JSON processor
  ```bash
  brew install jq  # macOS
  apt install jq   # Linux
  ```

- **MCP server binaries** - Build first:
  ```bash
  cd /path/to/mcp-agent-mail-rs
  cargo build --release -p mcp-stdio
  cargo build --release -p mcp-server
  ```

## Script Options

Most scripts support these options:

- `-h, --help` - Show help message
- `-p, --port PORT` - Custom server port (HTTP/SSE only)
- `-H, --host HOST` - Custom server host (HTTP/SSE only)

## Running Scripts

Make scripts executable first:
```bash
chmod +x *.sh
```

Then run:
```bash
./claude-code.sh
./cursor.sh
# etc.
```

## What Gets Configured

Each script configures the MCP server entry in the target client's config:

**STDIO mode** (default for most clients):
```json
{
  "mcpServers": {
    "mcp-agent-mail": {
      "command": "/path/to/am",
      "args": ["serve", "mcp", "--transport", "stdio"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

**SSE mode** (for clients that support it):
```json
{
  "mcpServers": {
    "mcp-agent-mail": {
      "type": "sse",
      "url": "http://127.0.0.1:8765/sse"
    }
  }
}
```

## Backup Policy

All scripts create timestamped backups before modifying existing configs:
```
config.json.backup.20231215120000
```

## Verification

Scripts verify installation by:
1. Checking config file exists
2. Validating JSON structure with jq
3. Confirming MCP server entry exists

For HTTP-based integrations:
4. Testing server connectivity on configured port

## Troubleshooting

### "MCP Agent Mail binary not found"
Build and install the project first:
```bash
cd /path/to/mcp-agent-mail-rs
cargo build --release -p mcp-agent-mail
# Or install globally:
cargo install --path crates/services/mcp-agent-mail
# Or install with 'am' alias:
make install-am
```

### "jq is required but not installed"
Install jq:
```bash
brew install jq  # macOS
apt install jq   # Linux
```

### "Client not detected"
The script will still create configuration files. Install the client and restart it to load the config.

### Permission Errors
Make scripts executable:
```bash
chmod +x *.sh
```

## Environment Variables

- `MCP_AGENT_MAIL_PORT` - Server port (default: 8765)
- `MCP_AGENT_MAIL_HOST` - Server host (default: 127.0.0.1)
- `RUST_LOG` - Log level (debug, info, warn, error)

## Testing Integration

After running an integration script:

1. **Restart the client** (IDE/CLI tool)
2. **Check server is running** (for HTTP mode):
   ```bash
   curl http://localhost:8765/health
   ```
3. **Test MCP tools** in the client:
   - "Register me as an agent"
   - "Check my inbox"
   - "List available MCP tools"

## MCP Tools Available

Once configured, clients have access to 28 MCP tools:

**Agent Management**: register, whois, list, update_profile
**Messaging**: send, reply, inbox_list, search, get, mark_read
**File Reservations**: paths, list, release, renew, force_release
**Project Management**: ensure, list, init_git
**Agent Links**: propose, approve, list, remove
**Build Slots**: acquire, list, release
**Contact Management**: add, list, approve, block
**Macros**: create, list, execute, delete


## Support

For issues or questions:
- Check project README: `/path/to/mcp-agent-mail-rs/README.md`
- Check client-specific documentation

## Contributing

To add a new integration script:

1. Follow the existing pattern (see claude-code.sh as template)
2. Include all standard sections: header, dependencies, detection, configuration, verification, summary
3. Add colored output using the standard color codes
4. Create backups before modifying configs
5. Handle both existing and new config files
6. Add documentation to this README
