#!/usr/bin/env bash
# continue.sh - Configure Continue.dev to use Mouchak Mail
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

# Continue.dev stores config in ~/.continue/config.json
CONTINUE_CONFIG="$HOME/.continue/config.json"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     Mouchak Mail - Continue.dev Integration             ${BLUE}║${NC}"
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

detect_continue() {
    log_info "Detecting Continue.dev..."

    # Check for VSCode with Continue extension
    if command -v code &> /dev/null; then
        log_success "Found VSCode in PATH"

        # Try to check if Continue is installed
        if code --list-extensions 2>/dev/null | grep -q "Continue.continue"; then
            log_success "Found Continue.dev extension installed"
        else
            log_warn "Continue.dev extension not detected"
            echo "  Install from: https://marketplace.visualstudio.com/items?itemName=Continue.continue"
        fi
    else
        log_warn "VSCode not found in PATH"
    fi

    # Check if config directory exists
    if [[ -d "$HOME/.continue" ]]; then
        log_success "Found Continue.dev config directory"
    else
        log_warn "Continue.dev config directory not found"
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

update_continue_config() {
    log_info "Updating Continue.dev config: $CONTINUE_CONFIG"

    # Create directory if needed
    mkdir -p "$(dirname "$CONTINUE_CONFIG")"

    # Create backup if file exists
    if [[ -f "$CONTINUE_CONFIG" ]]; then
        cp "$CONTINUE_CONFIG" "${CONTINUE_CONFIG}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing config"
    fi

    # Generate MCP server config
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
    if [[ -f "$CONTINUE_CONFIG" ]]; then
        # File exists - merge configuration
        local existing
        existing=$(cat "$CONTINUE_CONFIG")

        # Check if experimental.modelContextProtocolServers exists
        if echo "$existing" | jq -e '.experimental.modelContextProtocolServers' &> /dev/null; then
            echo "$existing" | jq --argjson config "$mcp_config" \
                ".experimental.modelContextProtocolServers[\"$MCP_SERVER_NAME\"] = \$config" > "$CONTINUE_CONFIG"
        elif echo "$existing" | jq -e '.experimental' &> /dev/null; then
            # experimental exists but not modelContextProtocolServers
            echo "$existing" | jq --argjson config "$mcp_config" \
                ".experimental.modelContextProtocolServers = {\"$MCP_SERVER_NAME\": \$config}" > "$CONTINUE_CONFIG"
        else
            # No experimental key
            echo "$existing" | jq --argjson config "$mcp_config" \
                ". + {experimental: {modelContextProtocolServers: {\"$MCP_SERVER_NAME\": \$config}}}" > "$CONTINUE_CONFIG"
        fi
    else
        # Create new config file
        jq -n --argjson config "$mcp_config" \
            "{experimental: {modelContextProtocolServers: {\"$MCP_SERVER_NAME\": \$config}}}" > "$CONTINUE_CONFIG"
    fi

    log_success "Updated $CONTINUE_CONFIG"
}

verify_installation() {
    log_info "Verifying installation..."

    if [[ -f "$CONTINUE_CONFIG" ]]; then
        if jq -e ".experimental.modelContextProtocolServers[\"$MCP_SERVER_NAME\"]" "$CONTINUE_CONFIG" &> /dev/null; then
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
    echo "  • Config: $CONTINUE_CONFIG"
    echo ""
    echo "Next steps:"
    echo "  1. Restart VSCode to load the new configuration"
    echo "  2. Open Continue.dev panel (Cmd+L / Ctrl+L)"
    echo "  3. Mouchak Mail tools will be available"
    echo "  4. Try: '@mouchak-mail register me as an agent'"
    echo ""
    echo "Continue.dev + Mouchak Mail capabilities:"
    echo "  • Context-aware agent registration"
    echo "  • Message-based coordination between agents"
    echo "  • File reservation to prevent conflicts"
    echo "  • Search communication history"
    echo "  • Link agents across projects"
    echo ""
    echo "Note: MCP support in Continue.dev is experimental."
    echo "Check https://continue.dev/docs for latest MCP documentation."
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure Continue.dev to use Mouchak Mail via stdio transport.

Options:
  -h, --help            Show this help message

Examples:
  $(basename "$0")                    # Install for Continue.dev

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
    detect_continue
    find_mcp_server
    update_continue_config
    verify_installation
    print_summary
}

main "$@"
