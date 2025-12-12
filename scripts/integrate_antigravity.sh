#!/usr/bin/env bash
# integrate_antigravity.sh - Configure Antigravity (Gemini Code) to use MCP Agent Mail
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

# Antigravity config location (Gemini Code Assistant)
ANTIGRAVITY_CONFIG="$HOME/.gemini/mcp_servers.json"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     MCP Agent Mail - Antigravity Integration               ${BLUE}║${NC}"
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

detect_antigravity() {
    log_info "Detecting Antigravity installation..."
    
    # Check for Antigravity/Gemini config directory
    if [[ -d "$HOME/.gemini" ]]; then
        log_success "Found Antigravity config directory: ~/.gemini"
        return 0
    fi
    
    # Check for VSCode Gemini extension
    local gemini_paths=(
        "$HOME/.vscode/extensions/google.gemini-"*
        "$HOME/.vscode-server/extensions/google.gemini-"*
    )
    
    for path in "${gemini_paths[@]}"; do
        if [[ -d "$path" ]]; then
            log_success "Found Gemini extension at: $path"
            return 0
        fi
    done
    
    log_warn "Antigravity not detected"
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
  },
  "description": "MCP Agent Mail - Agent-to-agent communication system"
}
EOF
}

update_antigravity_config() {
    log_info "Updating Antigravity config: $ANTIGRAVITY_CONFIG"
    
    mkdir -p "$(dirname "$ANTIGRAVITY_CONFIG")"
    
    if [[ -f "$ANTIGRAVITY_CONFIG" ]]; then
        cp "$ANTIGRAVITY_CONFIG" "${ANTIGRAVITY_CONFIG}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing config"
    fi
    
    local mcp_config
    mcp_config=$(generate_mcp_config)
    
    if [[ -f "$ANTIGRAVITY_CONFIG" ]]; then
        local existing
        existing=$(cat "$ANTIGRAVITY_CONFIG")
        
        if echo "$existing" | jq -e '.mcpServers' &> /dev/null; then
            echo "$existing" | jq --argjson config "$mcp_config" \
                ".mcpServers[\"$MCP_SERVER_NAME\"] = \$config" > "$ANTIGRAVITY_CONFIG"
        else
            echo "$existing" | jq --argjson config "$mcp_config" \
                ". + {mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$ANTIGRAVITY_CONFIG"
        fi
    else
        jq -n --argjson config "$mcp_config" \
            "{mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$ANTIGRAVITY_CONFIG"
    fi
    
    log_success "Updated $ANTIGRAVITY_CONFIG"
}

verify_installation() {
    log_info "Verifying installation..."
    
    if [[ -f "$ANTIGRAVITY_CONFIG" ]]; then
        if jq -e ".mcpServers[\"$MCP_SERVER_NAME\"]" "$ANTIGRAVITY_CONFIG" &> /dev/null; then
            log_success "Config verified: $ANTIGRAVITY_CONFIG"
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
    echo "  • Config: $ANTIGRAVITY_CONFIG"
    echo ""
    echo "Next steps:"
    echo "  1. Restart Antigravity to load the new configuration"
    echo "  2. MCP tools should now be available in your sessions"
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure Antigravity (Gemini Code) to use MCP Agent Mail.

Options:
  -P, --port PORT       MCP server port (default: 8765)
  -h, --help            Show this help message
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
    detect_antigravity
    find_mcp_server
    update_antigravity_config
    verify_installation
    print_summary
}

main "$@"
