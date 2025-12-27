#!/usr/bin/env bash
# integrate_opencode.sh - Configure OpenCode to use MCP Agent Mail
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

# OpenCode config location (go-based CLI tool)
OPENCODE_CONFIG="$HOME/.opencode/config.json"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     MCP Agent Mail - OpenCode Integration                  ${BLUE}║${NC}"
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

detect_opencode() {
    log_info "Detecting OpenCode installation..."
    
    if command -v opencode &> /dev/null; then
        log_success "Found OpenCode in PATH"
        return 0
    fi
    
    log_warn "OpenCode not detected"
    log_info "Install from: https://github.com/opencode-ai/opencode"
    log_info "Proceeding with config file creation anyway..."
    return 0
}

find_mcp_server() {
    log_info "Locating MCP Agent Mail binary..."
    
    if [[ -n "${MCP_SERVER_PATH:-}" ]] && [[ -x "$MCP_SERVER_PATH" ]]; then
        log_success "Using provided MCP_SERVER_PATH: $MCP_SERVER_PATH"
        return 0
    fi
    
    if command -v am &> /dev/null; then
        MCP_SERVER_PATH=$(command -v am)
        log_success "Found 'am' alias: $MCP_SERVER_PATH"
        return 0
    fi
    
    if command -v mcp-agent-mail &> /dev/null; then
        MCP_SERVER_PATH=$(command -v mcp-agent-mail)
        log_success "Found mcp-agent-mail: $MCP_SERVER_PATH"
        return 0
    fi
    
    local target_paths=(
        "$PROJECT_ROOT/target/release/mcp-agent-mail"
        "$PROJECT_ROOT/target/debug/mcp-agent-mail"
        "$HOME/.local/bin/am"
        "$HOME/.cargo/bin/mcp-agent-mail"
    )
    
    for path in "${target_paths[@]}"; do
        if [[ -x "$path" ]]; then
            MCP_SERVER_PATH="$path"
            log_success "Found MCP Agent Mail: $MCP_SERVER_PATH"
            return 0
        fi
    done
    
    log_error "MCP Agent Mail binary not found!"
    echo "  Install with: cargo install --path crates/services/mcp-agent-mail"
    return 1
}

generate_mcp_config() {
    cat <<EOF
{
  "command": "$MCP_SERVER_PATH",
  "args": ["serve", "mcp", "--transport", "stdio"],
  "env": {
    "RUST_LOG": "info"
  }
}
EOF
}

update_opencode_config() {
    log_info "Updating OpenCode config: $OPENCODE_CONFIG"
    
    mkdir -p "$(dirname "$OPENCODE_CONFIG")"
    
    if [[ -f "$OPENCODE_CONFIG" ]]; then
        cp "$OPENCODE_CONFIG" "${OPENCODE_CONFIG}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing config"
    fi
    
    local mcp_config
    mcp_config=$(generate_mcp_config)
    
    if [[ -f "$OPENCODE_CONFIG" ]]; then
        local existing
        existing=$(cat "$OPENCODE_CONFIG")
        
        if echo "$existing" | jq -e '.mcpServers' &> /dev/null; then
            echo "$existing" | jq --argjson config "$mcp_config" \
                ".mcpServers[\"$MCP_SERVER_NAME\"] = \$config" > "$OPENCODE_CONFIG"
        else
            echo "$existing" | jq --argjson config "$mcp_config" \
                ". + {mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$OPENCODE_CONFIG"
        fi
    else
        jq -n --argjson config "$mcp_config" \
            "{mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$OPENCODE_CONFIG"
    fi
    
    log_success "Updated $OPENCODE_CONFIG"
}

verify_installation() {
    log_info "Verifying installation..."
    
    if [[ -f "$OPENCODE_CONFIG" ]]; then
        if jq -e ".mcpServers[\"$MCP_SERVER_NAME\"]" "$OPENCODE_CONFIG" &> /dev/null; then
            log_success "Config verified: $OPENCODE_CONFIG"
        else
            log_warn "MCP server not found in config"
        fi
    fi
    
    if curl -s "http://$MCP_SERVER_HOST:$MCP_SERVER_PORT/api/health" &> /dev/null; then
        log_success "MCP Agent Mail server is running on port $MCP_SERVER_PORT"
    else
        log_info "Server not running (STDIO mode will start automatically)"
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
    echo "  • Port: $MCP_SERVER_PORT"
    echo "  • Config: $OPENCODE_CONFIG"
    echo ""
    echo "Next steps:"
    echo "  1. Restart OpenCode to load the new configuration"
    echo "  2. STDIO mode: OpenCode will spawn the server automatically"
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure OpenCode to use MCP Agent Mail.

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
    detect_opencode
    
    if ! find_mcp_server; then
        log_error "Cannot proceed without MCP Agent Mail binary"
        exit 1
    fi
    
    update_opencode_config
    verify_installation
    print_summary
}

main "$@"
