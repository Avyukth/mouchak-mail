#!/usr/bin/env bash
# copilot.sh - Configure GitHub Copilot to use Mouchak Mail
# Part of mouchak-mail integration scripts

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

MCP_SERVER_NAME="mouchak-mail"
MCP_SERVER_PORT="${MOUCHAK_MAIL_PORT:-8765}"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     Mouchak Mail - GitHub Copilot Integration           ${BLUE}║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

check_dependencies() {
    log_info "Checking dependencies..."

    if ! command -v gh &> /dev/null; then
        log_warn "GitHub CLI (gh) not found"
        echo "  Install from: https://cli.github.com"
        echo "  This is optional for Copilot integration"
    else
        log_success "Found GitHub CLI"
    fi
}

detect_copilot() {
    log_info "Detecting GitHub Copilot..."

    # Check for VSCode with Copilot
    if command -v code &> /dev/null; then
        log_success "Found VSCode in PATH"

        # Check for Copilot extensions
        local has_copilot=false
        if code --list-extensions 2>/dev/null | grep -q "GitHub.copilot"; then
            log_success "Found GitHub Copilot extension"
            has_copilot=true
        fi
        if code --list-extensions 2>/dev/null | grep -q "GitHub.copilot-chat"; then
            log_success "Found GitHub Copilot Chat extension"
            has_copilot=true
        fi

        if [[ "$has_copilot" == false ]]; then
            log_warn "GitHub Copilot not detected"
            echo "  Install from: https://marketplace.visualstudio.com/items?itemName=GitHub.copilot"
        fi
    else
        log_warn "VSCode not found in PATH"
    fi

    return 0
}

find_mcp_server() {
    log_info "Locating MCP server binary..."

    # Check if mcp-server (HTTP) is in PATH
    if command -v mcp-server &> /dev/null; then
        MCP_SERVER_PATH=$(which mcp-server)
        log_success "Found mcp-server: $MCP_SERVER_PATH"
        SERVER_TYPE="http"
        return 0
    fi

    # Check project build directories
    local target_paths=(
        "$PROJECT_ROOT/target/release/mcp-server"
        "$PROJECT_ROOT/target/debug/mcp-server"
    )

    for path in "${target_paths[@]}"; do
        if [[ -x "$path" ]]; then
            MCP_SERVER_PATH="$path"
            log_success "Found mcp-server: $MCP_SERVER_PATH"
            SERVER_TYPE="http"
            return 0
        fi
    done

    log_error "mcp-server not found!"
    echo "  Build it with: cd $PROJECT_ROOT && cargo build --release -p mcp-server"
    exit 1
}

create_workspace_config() {
    log_info "Creating workspace configuration for Copilot..."

    # Create .vscode directory if it doesn't exist
    mkdir -p .vscode

    local settings_file=".vscode/settings.json"

    # Create backup if file exists
    if [[ -f "$settings_file" ]]; then
        cp "$settings_file" "${settings_file}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing settings"
    fi

    # Add instructions for Copilot to use the API
    cat > "$settings_file" <<EOF
{
  "github.copilot.advanced": {
    "debug.overrideEngine": "gpt-4",
    "debug.testOverrideProxyUrl": "http://localhost:$MCP_SERVER_PORT"
  },
  "github.copilot.chat.codeGeneration.instructions": [
    {
      "text": "Mouchak Mail is available at http://localhost:$MCP_SERVER_PORT. Use it for:",
      "file": ".copilot-instructions.md"
    }
  ]
}
EOF

    # Create instructions file
    cat > ".copilot-instructions.md" <<EOF
# Mouchak Mail Integration

This project uses Mouchak Mail for multi-agent coordination.

## Available via HTTP API

Base URL: http://localhost:$MCP_SERVER_PORT

## Key Endpoints

- POST /api/agent/register - Register as an agent
- POST /api/message/send - Send messages to other agents
- POST /api/inbox - Check your inbox
- POST /api/file_reservations/paths - Reserve files before editing
- POST /api/messages/search - Search message history

## Usage Pattern

1. Register yourself: POST to /api/agent/register
2. Before editing files: POST to /api/file_reservations/paths
3. Send updates: POST to /api/message/send
4. Check for messages: POST to /api/inbox
5. Release files: POST to /api/file_reservations/release

See full API docs at: $PROJECT_ROOT/docs/API.md
EOF

    log_success "Created workspace configuration"
}

create_startup_script() {
    log_info "Creating MCP server startup helper..."

    cat > "start-mcp-mail.sh" <<EOF
#!/usr/bin/env bash
# Start Mouchak Mail server for GitHub Copilot integration

echo "Starting Mouchak Mail server..."
"$MCP_SERVER_PATH" &
MCP_PID=\$!

echo "Mouchak Mail server started (PID: \$MCP_PID)"
echo "Server running at: http://localhost:$MCP_SERVER_PORT"
echo "Press Ctrl+C to stop"

cleanup() {
    echo "Stopping Mouchak Mail server..."
    kill \$MCP_PID 2>/dev/null || true
}

trap cleanup EXIT
wait \$MCP_PID
EOF

    chmod +x "start-mcp-mail.sh"
    log_success "Created startup script: start-mcp-mail.sh"
}

print_summary() {
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}     Integration Complete!                                   ${GREEN}║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Configuration:"
    echo "  • Server: $MCP_SERVER_NAME"
    echo "  • Binary: $MCP_SERVER_PATH"
    echo "  • Port: $MCP_SERVER_PORT"
    echo "  • Type: HTTP REST API"
    echo ""
    echo "Files created:"
    echo "  • .vscode/settings.json - VSCode workspace settings"
    echo "  • .copilot-instructions.md - Instructions for Copilot"
    echo "  • start-mcp-mail.sh - Server startup script"
    echo ""
    echo "How to use:"
    echo "  1. Start the server: ./start-mcp-mail.sh"
    echo "  2. Open VSCode with Copilot"
    echo "  3. Use Copilot Chat with these prompts:"
    echo ""
    echo "Example prompts:"
    echo "  'Register me as an agent using the Mouchak Mail API'"
    echo "  'Check my Mouchak Mail inbox via the API'"
    echo "  'Send a message to agent BlueOcean using Mouchak Mail'"
    echo "  'Reserve src/**/*.ts for editing via Mouchak Mail'"
    echo ""
    echo "Note: GitHub Copilot doesn't have native MCP support."
    echo "This integration uses the HTTP REST API with instructions"
    echo "in .copilot-instructions.md to guide Copilot's responses."
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure GitHub Copilot to use Mouchak Mail via HTTP API.

Options:
  -p, --port PORT       MCP server port (default: 8765)
  -h, --help            Show this help message

Examples:
  $(basename "$0")                    # Install for Copilot
  $(basename "$0") --port 9000        # Use custom port

Note: Copilot doesn't have native MCP support. This creates
workspace configuration and instructions to guide Copilot.

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -p|--port)
            MCP_SERVER_PORT="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Main execution
main() {
    print_header
    check_dependencies
    detect_copilot
    find_mcp_server
    create_workspace_config
    create_startup_script
    print_summary
}

main "$@"
