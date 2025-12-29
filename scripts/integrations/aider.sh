#!/usr/bin/env bash
# aider.sh - Configure Aider to use Mouchak Mail
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
AIDER_CONFIG="$HOME/.aider.conf.yml"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     Mouchak Mail - Aider Integration                     ${BLUE}║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

check_dependencies() {
    log_info "Checking dependencies..."

    # Check for aider
    if ! command -v aider &> /dev/null; then
        log_warn "aider not found in PATH"
        echo "  Install with: pip install aider-chat"
        echo "  Or: pipx install aider-chat"
    else
        AIDER_VERSION=$(aider --version 2>/dev/null | head -n1 || echo "unknown")
        log_success "Found aider: $AIDER_VERSION"
    fi
}

detect_aider() {
    log_info "Detecting Aider installation..."

    if command -v aider &> /dev/null; then
        log_success "Found Aider in PATH"
        return 0
    fi

    log_warn "Aider not found"
    log_info "Install from: https://aider.chat"
    return 0
}

find_mcp_server() {
    log_info "Locating mcp-stdio-server binary..."

    # Check if in PATH
    if command -v mcp-stdio-server &> /dev/null; then
        MCP_SERVER_PATH=$(which mcp-stdio-server)
        log_success "Found mcp-stdio-server: $MCP_SERVER_PATH"
        return 0
    fi

    # Check project build directories
    local target_paths=(
        "$PROJECT_ROOT/target/release/mcp-stdio-server"
        "$PROJECT_ROOT/target/debug/mcp-stdio-server"
    )

    for path in "${target_paths[@]}"; do
        if [[ -x "$path" ]]; then
            MCP_SERVER_PATH="$path"
            log_success "Found mcp-stdio-server: $MCP_SERVER_PATH"
            return 0
        fi
    done

    log_error "mcp-stdio-server not found!"
    echo "  Build it with: cd $PROJECT_ROOT && cargo build --release -p mcp-stdio"
    exit 1
}

create_aider_wrapper() {
    log_info "Creating Aider wrapper script with Mouchak Mail..."

    local wrapper_script="$HOME/.local/bin/aider-with-mail"

    # Create directory if needed
    mkdir -p "$(dirname "$wrapper_script")"

    # Create wrapper script
    cat > "$wrapper_script" <<'EOF'
#!/usr/bin/env bash
# Aider wrapper with Mouchak Mail integration

# Start mcp-stdio-server in background
MCP_SERVER_PATH="__MCP_SERVER_PATH__"
MCP_PID_FILE="/tmp/mouchak-mail.pid"

cleanup() {
    if [[ -f "$MCP_PID_FILE" ]]; then
        kill $(cat "$MCP_PID_FILE") 2>/dev/null || true
        rm -f "$MCP_PID_FILE"
    fi
}

trap cleanup EXIT

# Start MCP server if not already running
if [[ ! -f "$MCP_PID_FILE" ]] || ! kill -0 $(cat "$MCP_PID_FILE") 2>/dev/null; then
    echo "Starting Mouchak Mail server..."
    "$MCP_SERVER_PATH" &
    echo $! > "$MCP_PID_FILE"
    sleep 1
fi

# Export environment for aider to access MCP tools
export MOUCHAK_MAIL_ENABLED=1
export MCP_SERVER_PATH="$MCP_SERVER_PATH"

# Run aider with all arguments
exec aider "$@"
EOF

    # Replace placeholder with actual path
    sed -i.bak "s|__MCP_SERVER_PATH__|$MCP_SERVER_PATH|g" "$wrapper_script"
    rm -f "${wrapper_script}.bak"

    # Make executable
    chmod +x "$wrapper_script"

    log_success "Created wrapper script: $wrapper_script"
}

update_aider_config() {
    log_info "Creating Aider configuration..."

    # Create backup if file exists
    if [[ -f "$AIDER_CONFIG" ]]; then
        cp "$AIDER_CONFIG" "${AIDER_CONFIG}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing config"
    fi

    # Add MCP-related configuration to .aider.conf.yml
    cat >> "$AIDER_CONFIG" <<EOF

# Mouchak Mail integration
# Use the wrapper script: aider-with-mail
# Or set environment: export MOUCHAK_MAIL_ENABLED=1

# Add this to your prompts to use Mouchak Mail:
# - Register as agent: "Register me as an agent named <Name> in this project"
# - Check inbox: "Check my Mouchak Mail inbox"
# - Send message: "Send a message to agent <Name> about <topic>"
# - Reserve files: "Reserve files in src/ for editing"

EOF

    log_success "Updated $AIDER_CONFIG"
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
    echo "  • Wrapper: $HOME/.local/bin/aider-with-mail"
    echo "  • Config: $AIDER_CONFIG"
    echo ""
    echo "How to use:"
    echo "  Option 1 - Use wrapper script:"
    echo "    aider-with-mail"
    echo ""
    echo "  Option 2 - Manual setup:"
    echo "    $MCP_SERVER_PATH &"
    echo "    aider"
    echo ""
    echo "Example prompts for Aider:"
    echo "  'Register me as an agent named BlueOcean in this project'"
    echo "  'Check my Mouchak Mail inbox for messages'"
    echo "  'Send a message to GreenForest about the refactor'"
    echo "  'Reserve src/**/*.rs for editing'"
    echo "  'Search Mouchak Mail for messages about authentication'"
    echo ""
    echo "Note: Aider doesn't have native MCP support yet."
    echo "The wrapper script starts the MCP server, and you can use"
    echo "direct HTTP requests via curl or the mcp-cli tool."
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure Aider to use Mouchak Mail.

Options:
  -h, --help            Show this help message

Examples:
  $(basename "$0")                    # Install for Aider

Note: Aider doesn't have native MCP support. This script creates
a wrapper that starts the MCP server alongside Aider.

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
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
    detect_aider
    find_mcp_server
    create_aider_wrapper
    update_aider_config
    print_summary
}

main "$@"
