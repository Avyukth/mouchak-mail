# Quick Start Guide - Mouchak Mail Integrations

Choose your AI coding tool and run the corresponding script:

## 🚀 One-Command Installation

```bash
# Make scripts executable (one-time)
chmod +x *.sh

# Then run the script for your tool:
```

### Claude Code CLI
```bash
./claude-code.sh
```
**What it does**: Updates `~/.claude/settings.json` with STDIO MCP server config

---

### Cursor IDE
```bash
./cursor.sh
```
**What it does**: Adds MCP server to `~/.cursor/mcp_settings.json`

---

### Cline (VSCode Extension)
```bash
./cline.sh
```
**What it does**: Configures VSCode's Cline extension global storage

---

### Windsurf IDE
```bash
./windsurf.sh
```
**What it does**: Updates `~/.codeium/windsurf/mcp_config.json`

---

### Aider CLI
```bash
./aider.sh
```
**What it does**: Creates `aider-with-mail` wrapper script at `~/.local/bin/aider-with-mail`

**Usage**: Run `aider-with-mail` instead of `aider`

---

### Continue.dev (VSCode Extension)
```bash
./continue.sh
```
**What it does**: Adds to `~/.continue/config.json` experimental MCP servers

**Note**: MCP support is experimental in Continue.dev

---

### GitHub Copilot
```bash
./copilot.sh
```
**What it does**: Creates workspace `.vscode/settings.json` and `.copilot-instructions.md`

**Usage**: Run `./start-mcp-mail.sh` to start the server, then use Copilot

---

### Generic MCP Client
```bash
./generic-mcp.sh
```
**What it does**: Generates `mouchak-mail-configs/` with example configs for all transport modes

**Use this if**: Your client isn't listed above but supports MCP protocol

---

## Prerequisites

Before running any script:

1. **Install jq**:
   ```bash
   brew install jq  # macOS
   apt install jq   # Linux
   ```

2. **Build MCP servers**:
   ```bash
   cd /path/to/mouchak-mail
   cargo build --release -p mcp-stdio
   cargo build --release -p mcp-server
   ```

## After Installation

1. **Restart your client** (IDE/CLI tool)
2. **Start coding** - MCP tools are now available!
3. **Try these prompts**:
   - "Register me as an agent in this project"
   - "Check my Mouchak Mail inbox"
   - "Reserve src/ for editing"
   - "Send a message to other agents about this change"

## Troubleshooting

**Script says "not found"?**
- The tool will still create configs - just install the client and restart

**Permission error?**
```bash
chmod +x *.sh
```

**Server not starting?**
```bash
# For HTTP mode:
cargo run -p mcp-server

# For STDIO mode:
# Server starts automatically by the client
```

## What MCP Tools Are Available?

Once configured, you get 28 tools including:

- 🤖 **Agent Management**: register, whois, list
- 📧 **Messaging**: send, reply, inbox, search
- 🔒 **File Reservations**: reserve, release, renew
- 📁 **Projects**: ensure, list, init
- 🔗 **Linking**: cross-project agent connections
- 🛠️ **Build Slots**: exclusive build locks
- 📝 **Macros**: reusable workflows

## Need Help?

See [README.md](./README.md) for detailed documentation on each script.
