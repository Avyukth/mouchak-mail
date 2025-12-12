#!/usr/bin/env bash
# integrate_windsurf.sh - Configure Windsurf IDE to use MCP Agent Mail
# Part of mcp-agent-mail-rs integration scripts

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Default configuration
MCP_SERVER_PORT="${MCP_AGENT_MAIL_PORT:-8765}"
MCP_SERVER_HOST="${MCP_AGENT_MAIL_HOST:-127.0.0.1}"
MCP_SERVER_NAME="mcp-agent-mail"

# Windsurf config location
WINDSURF_CONFIG="$HOME/.codeium/windsurf/mcp_config.json"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     MCP Agent Mail - Windsurf IDE Integration              ${BLUE}║${NC}"
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
    log_info "Detecting Windsurf installation..."
    
    # Check common Windsurf paths
    local windsurf_paths=(
        "/Applications/Windsurf.app"
        "$HOME/Applications/Windsurf.app"
        "$HOME/.codeium/windsurf"
    )
    
    for path in "${windsurf_paths[@]}"; do
        if [[ -e "$path" ]]; then
            log_success "Found Windsurf at: $path"
            return 0
        fi
    done
    
    log_warn "Windsurf IDE not detected"
    log_info "Proceeding with config file creation anyway..."
    return 0
}

find_mcp_server() {
    log_info "Locating MCP server binary..."
    
    if command -v mcp-server &> /dev/null; then
        MCP_SERVER_PATH=$(which mcp-server)
        log_success "Found mcp-server: $MCP_SERVER_PATH"
        return 0
    fi
    
    local target_paths=(
        "$PROJECT_ROOT/target/release/mcp-server"
        "$PROJECT_ROOT/target/debug/mcp-server"
    )
    
    for path in "${target_paths[@]}"; do
        if [[ -x "$path" ]]; then
            MCP_SERVER_PATH="$path"
            log_success "Found mcp-server: $MCP_SERVER_PATH"
            return 0
        fi
    done
    
    MCP_SERVER_PATH="mcp-server"
    log_warn "mcp-server not found, using 'mcp-server' (must be in PATH)"
    return 0
}

generate_mcp_config() {
    cat <<EOF
{
  "command": "$MCP_SERVER_PATH",
  "args": ["--stdio"],
  "env": {
    "MCP_AGENT_MAIL_PORT": "$MCP_SERVER_PORT",
    "RUST_LOG": "info"
  }
}
EOF
}

update_windsurf_config() {
    log_info "Updating Windsurf config: $WINDSURF_CONFIG"
    
    # Create parent directory if needed
    mkdir -p "$(dirname "$WINDSURF_CONFIG")"
    
    # Create backup if file exists
    if [[ -f "$WINDSURF_CONFIG" ]]; then
        cp "$WINDSURF_CONFIG" "${WINDSURF_CONFIG}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing config"
    fi
    
    local mcp_config
    mcp_config=$(generate_mcp_config)
    
    if [[ -f "$WINDSURF_CONFIG" ]]; then
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
        jq -n --argjson config "$mcp_config" \
            "{mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$WINDSURF_CONFIG"
    fi
    
    log_success "Updated $WINDSURF_CONFIG"
}

verify_installation() {
    log_info "Verifying installation..."
    
    if [[ -f "$WINDSURF_CONFIG" ]]; then
        if jq -e ".mcpServers[\"$MCP_SERVER_NAME\"]" "$WINDSURF_CONFIG" &> /dev/null; then
            log_success "Config verified: $WINDSURF_CONFIG"
        else
            log_warn "MCP server not found in config"
        fi
    fi
    
    if curl -s "http://$MCP_SERVER_HOST:$MCP_SERVER_PORT/api/health" &> /dev/null; then
        log_success "MCP Agent Mail server is running on port $MCP_SERVER_PORT"
    else
        log_warn "MCP Agent Mail server not responding on port $MCP_SERVER_PORT"
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
    echo "  • Port: $MCP_SERVER_PORT"
    echo "  • Config: $WINDSURF_CONFIG"
    echo ""
    echo "Next steps:"
    echo "  1. Restart Windsurf IDE to load the new configuration"
    echo "  2. Click the hammer icon in Cascade toolbar to verify"
    echo "  3. MCP tools should appear in Windsurf's Cascade"
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure Windsurf IDE to use MCP Agent Mail.

Options:
  -P, --port PORT       MCP server port (default: 8765)
  -h, --help            Show this help message

Examples:
  $(basename "$0")                    # Default configuration
  $(basename "$0") --port 9000        # Use custom port
EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -P|--port)
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
