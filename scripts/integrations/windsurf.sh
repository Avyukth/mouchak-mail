#!/usr/bin/env bash
# windsurf.sh - Configure Windsurf IDE to use Mouchak Mail
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
WINDSURF_CONFIG="$HOME/.codeium/windsurf/mcp_config.json"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     Mouchak Mail - Windsurf IDE Integration             ${BLUE}║${NC}"
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

detect_windsurf() {
    log_info "Detecting Windsurf IDE..."

    # Check common Windsurf paths
    local windsurf_paths=(
        "/Applications/Windsurf.app"
        "$HOME/Applications/Windsurf.app"
    )

    for path in "${windsurf_paths[@]}"; do
        if [[ -e "$path" ]]; then
            log_success "Found Windsurf at: $path"
            return 0
        fi
    done

    log_warn "Windsurf IDE not detected"
    log_info "Download from: https://codeium.com/windsurf"
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

update_windsurf_config() {
    log_info "Updating Windsurf MCP config: $WINDSURF_CONFIG"

    # Create directory if needed
    mkdir -p "$(dirname "$WINDSURF_CONFIG")"

    # Create backup if file exists
    if [[ -f "$WINDSURF_CONFIG" ]]; then
        cp "$WINDSURF_CONFIG" "${WINDSURF_CONFIG}.backup.$(date +%Y%m%d%H%M%S)"
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
    if [[ -f "$WINDSURF_CONFIG" ]]; then
        # File exists - merge configuration
        local existing
        existing=$(cat "$WINDSURF_CONFIG")

        if echo "$existing" | jq -e '.mcpServers' &> /dev/null; then
            echo "$existing" | jq --argjson config "$mcp_config" \
                ".mcpServers[\"$MCP_SERVER_NAME\"] = \$config" > "$WINDSURF_CONFIG"
        else
            echo "$existing" | jq --argjson config "$mcp_config" \
                ". + {mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$WINDSURF_CONFIG"
        fi
    else
        # Create new config file
        jq -n --argjson config "$mcp_config" \
            "{mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$WINDSURF_CONFIG"
    fi

    log_success "Updated $WINDSURF_CONFIG"
}

verify_installation() {
    log_info "Verifying installation..."

    if [[ -f "$WINDSURF_CONFIG" ]]; then
        if jq -e ".mcpServers[\"$MCP_SERVER_NAME\"]" "$WINDSURF_CONFIG" &> /dev/null; then
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
    echo "  • Config: $WINDSURF_CONFIG"
    echo ""
    echo "Next steps:"
    echo "  1. Restart Windsurf IDE to load the new configuration"
    echo "  2. Open Windsurf's Cascade AI panel"
    echo "  3. Mouchak Mail tools will be available"
    echo "  4. Try: 'Show me how to use Mouchak Mail'"
    echo ""
    echo "Use cases for Windsurf + Mouchak Mail:"
    echo "  • Multi-agent coordination on large refactors"
    echo "  • Backend/frontend agent communication"
    echo "  • File locking to prevent edit conflicts"
    echo "  • Persistent message history across sessions"
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure Windsurf IDE to use Mouchak Mail via stdio transport.

Options:
  -h, --help            Show this help message

Examples:
  $(basename "$0")                    # Install for Windsurf IDE

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
    detect_windsurf
    find_mcp_server
    update_windsurf_config
    verify_installation
    print_summary
}

main "$@"
