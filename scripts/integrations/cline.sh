#!/usr/bin/env bash
# cline.sh - Configure Cline VSCode extension to use Mouchak Mail
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

# Cline stores MCP settings in VSCode's global storage
# Platform-specific paths
if [[ "$OSTYPE" == "darwin"* ]]; then
    CLINE_CONFIG="$HOME/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    CLINE_CONFIG="$HOME/.config/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json"
else
    CLINE_CONFIG="$HOME/.vscode/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json"
fi

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     Mouchak Mail - Cline Extension Integration          ${BLUE}║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

check_dependencies() {
    log_info "Checking dependencies..."

    if ! command -v jq &> /dev/null; then
        log_error "jq is required but not installed."
        echo "  Install with: brew install jq (macOS) or apt install jq (Linux)"
        exit 1
    fi

    log_success "Dependencies satisfied"
}

detect_cline() {
    log_info "Detecting Cline extension..."

    # Check for VSCode
    if command -v code &> /dev/null; then
        log_success "Found VSCode in PATH"

        # Try to check if Cline is installed
        if code --list-extensions 2>/dev/null | grep -q "saoudrizwan.claude-dev"; then
            log_success "Found Cline extension installed"
        else
            log_warn "Cline extension not detected"
            echo "  Install from: https://marketplace.visualstudio.com/items?itemName=saoudrizwan.claude-dev"
        fi
    else
        log_warn "VSCode not found in PATH"
        log_info "Install from: https://code.visualstudio.com"
    fi

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

update_cline_config() {
    log_info "Updating Cline MCP settings: $CLINE_CONFIG"

    # Create directory if needed
    mkdir -p "$(dirname "$CLINE_CONFIG")"

    # Create backup if file exists
    if [[ -f "$CLINE_CONFIG" ]]; then
        cp "$CLINE_CONFIG" "${CLINE_CONFIG}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing config"
    fi

    # Generate MCP server config for stdio transport
    local mcp_config
    mcp_config=$(cat <<EOF
{
  "command": "$MCP_SERVER_PATH",
  "args": [],
  "env": {
    "RUST_LOG": "info"
  }
}
EOF
)

    # Create or update config file
    if [[ -f "$CLINE_CONFIG" ]]; then
        # File exists - merge configuration
        local existing
        existing=$(cat "$CLINE_CONFIG")

        if echo "$existing" | jq -e '.mcpServers' &> /dev/null; then
            echo "$existing" | jq --argjson config "$mcp_config" \
                ".mcpServers[\"$MCP_SERVER_NAME\"] = \$config" > "$CLINE_CONFIG"
        else
            echo "$existing" | jq --argjson config "$mcp_config" \
                ". + {mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$CLINE_CONFIG"
        fi
    else
        # Create new config file
        jq -n --argjson config "$mcp_config" \
            "{mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$CLINE_CONFIG"
    fi

    log_success "Updated $CLINE_CONFIG"
}

verify_installation() {
    log_info "Verifying installation..."

    if [[ -f "$CLINE_CONFIG" ]]; then
        if jq -e ".mcpServers[\"$MCP_SERVER_NAME\"]" "$CLINE_CONFIG" &> /dev/null; then
            log_success "Configuration verified"
        else
            log_warn "MCP server not found in config"
        fi
    fi
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
    echo "  • Config: $CLINE_CONFIG"
    echo ""
    echo "Next steps:"
    echo "  1. Restart VSCode to load the new configuration"
    echo "  2. Open the Cline panel in VSCode (Cmd+Shift+P > 'Cline: Open')"
    echo "  3. Mouchak Mail tools will be available to Cline"
    echo "  4. Try: 'Register me as an agent and check my inbox'"
    echo ""
    echo "What Cline can do with Mouchak Mail:"
    echo "  • Register as an agent in your project"
    echo "  • Send messages to other agents about work"
    echo "  • Reserve files before editing"
    echo "  • Check inbox for messages from other agents"
    echo "  • Search message history"
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure Cline VSCode extension to use Mouchak Mail via stdio transport.

Options:
  -h, --help            Show this help message

Examples:
  $(basename "$0")                    # Install for Cline

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
    detect_cline
    find_mcp_server
    update_cline_config
    verify_installation
    print_summary
}

main "$@"
