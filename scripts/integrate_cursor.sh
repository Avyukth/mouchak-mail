#!/usr/bin/env bash
# integrate_cursor.sh - Configure Cursor IDE to use MCP Agent Mail
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

# Config file locations
CURSOR_CONFIG_GLOBAL="$HOME/.cursor/mcp.json"
CURSOR_CONFIG_PROJECT=".cursor/mcp.json"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     MCP Agent Mail - Cursor IDE Integration                ${BLUE}║${NC}"
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

detect_cursor() {
    log_info "Detecting Cursor installation..."
    
    # Check common Cursor paths
    local cursor_paths=(
        "/Applications/Cursor.app"
        "$HOME/Applications/Cursor.app"
        "/usr/share/applications/cursor.desktop"
    )
    
    for path in "${cursor_paths[@]}"; do
        if [[ -e "$path" ]]; then
            log_success "Found Cursor at: $path"
            return 0
        fi
    done
    
    # Check if cursor command exists
    if command -v cursor &> /dev/null; then
        log_success "Found Cursor CLI in PATH"
        return 0
    fi
    
    log_warn "Cursor IDE not detected"
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

ensure_server_running() {
    log_info "Checking if MCP Agent Mail server is running..."
    
    if curl -s "http://$MCP_SERVER_HOST:$MCP_SERVER_PORT/api/health" &> /dev/null; then
        log_success "Server is running on port $MCP_SERVER_PORT"
        return 0
    fi
    
    log_warn "Server not running on port $MCP_SERVER_PORT"
    
    if [[ -z "$MCP_SERVER_PATH" ]]; then
        return 1
    fi
    
    log_info "Starting MCP Agent Mail server in background..."
    nohup "$MCP_SERVER_PATH" serve http --port "$MCP_SERVER_PORT" > /dev/null 2>&1 &
    sleep 2
    
    if curl -s "http://$MCP_SERVER_HOST:$MCP_SERVER_PORT/api/health" &> /dev/null; then
        log_success "Server started successfully"
    fi
    return 0
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

update_cursor_config() {
    local config_file="$1"
    local scope="$2"
    
    log_info "Updating $scope config: $config_file"
    
    # Create parent directory if needed
    mkdir -p "$(dirname "$config_file")"
    
    # Create backup if file exists
    if [[ -f "$config_file" ]]; then
        cp "$config_file" "${config_file}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing config"
    fi
    
    local mcp_config
    mcp_config=$(generate_mcp_config)
    
    if [[ -f "$config_file" ]]; then
        local existing
        existing=$(cat "$config_file")
        
        if echo "$existing" | jq -e '.mcpServers' &> /dev/null; then
            echo "$existing" | jq --argjson config "$mcp_config" \
                ".mcpServers[\"$MCP_SERVER_NAME\"] = \$config" > "$config_file"
        else
            echo "$existing" | jq --argjson config "$mcp_config" \
                ". + {mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$config_file"
        fi
    else
        jq -n --argjson config "$mcp_config" \
            "{mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$config_file"
    fi
    
    log_success "Updated $config_file"
}

setup_global_scope() {
    log_info "Setting up global Cursor integration..."
    update_cursor_config "$CURSOR_CONFIG_GLOBAL" "global"
}

setup_project_scope() {
    local project_dir="${1:-.}"
    local config_file="$project_dir/$CURSOR_CONFIG_PROJECT"
    
    log_info "Setting up project-scope integration in $project_dir..."
    update_cursor_config "$config_file" "project"
}

verify_installation() {
    log_info "Verifying installation..."
    
    if [[ -f "$CURSOR_CONFIG_GLOBAL" ]]; then
        if jq -e ".mcpServers[\"$MCP_SERVER_NAME\"]" "$CURSOR_CONFIG_GLOBAL" &> /dev/null; then
            log_success "Global config verified: $CURSOR_CONFIG_GLOBAL"
        else
            log_warn "MCP server not found in global config"
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
    echo ""
    echo "Next steps:"
    echo "  1. Restart Cursor IDE to load the new configuration"
    echo "  2. STDIO mode: Cursor will spawn the server automatically"
    echo "  3. MCP tools should appear in Cursor's Composer"
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure Cursor IDE to use MCP Agent Mail.

Options:
  -s, --scope SCOPE     Config scope: global (default) or project
  -p, --project DIR     Project directory for project-scope config
  -P, --port PORT       MCP server port (default: 8765)
  -h, --help            Show this help message

Examples:
  $(basename "$0")                           # Global scope
  $(basename "$0") --scope project           # Project scope in current dir
  $(basename "$0") --project /path/to/proj   # Project scope in specific dir
EOF
}

# Parse arguments
SCOPE="global"
PROJECT_DIR="."

while [[ $# -gt 0 ]]; do
    case $1 in
        -s|--scope)
            SCOPE="$2"
            shift 2
            ;;
        -p|--project)
            PROJECT_DIR="$2"
            SCOPE="project"
            shift 2
            ;;
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
    detect_cursor
    
    if ! find_mcp_server; then
        log_error "Cannot proceed without MCP Agent Mail binary"
        exit 1
    fi
    
    if [[ "$SCOPE" == "project" ]]; then
        setup_project_scope "$PROJECT_DIR"
    else
        setup_global_scope
    fi
    
    verify_installation
    print_summary
}

main "$@"
