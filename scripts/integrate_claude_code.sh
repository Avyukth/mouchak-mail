#!/usr/bin/env bash
# integrate_claude_code.sh - Configure Claude Code to use MCP Agent Mail
# Part of mcp-agent-mail-rs integration scripts

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Default configuration
MCP_SERVER_PORT="${MCP_AGENT_MAIL_PORT:-8765}"
MCP_SERVER_HOST="${MCP_AGENT_MAIL_HOST:-127.0.0.1}"
MCP_SERVER_NAME="mcp-agent-mail"

# Config file locations
CLAUDE_CONFIG_USER="$HOME/.claude.json"
CLAUDE_CONFIG_PROJECT=".mcp.json"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     MCP Agent Mail - Claude Code Integration               ${BLUE}║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

check_dependencies() {
    log_info "Checking dependencies..."
    
    # Check for jq (JSON processor)
    if ! command -v jq &> /dev/null; then
        log_error "jq is required but not installed."
        echo "  Install with: brew install jq (macOS) or apt install jq (Linux)"
        exit 1
    fi
    
    log_success "Dependencies satisfied"
}

detect_claude_code() {
    log_info "Detecting Claude Code installation..."
    
    # Check if claude CLI is available
    if command -v claude &> /dev/null; then
        CLAUDE_VERSION=$(claude --version 2>/dev/null || echo "unknown")
        log_success "Found Claude Code: $CLAUDE_VERSION"
        return 0
    fi
    
    # Check common installation paths
    local claude_paths=(
        "/usr/local/bin/claude"
        "$HOME/.local/bin/claude"
        "$HOME/.claude/bin/claude"
    )
    
    for path in "${claude_paths[@]}"; do
        if [[ -x "$path" ]]; then
            log_success "Found Claude Code at: $path"
            return 0
        fi
    done
    
    log_warn "Claude Code CLI not found in PATH"
    log_info "Proceeding with config file creation anyway..."
    return 0
}

find_mcp_server() {
    log_info "Locating MCP server binary..."
    
    # Check if mcp-server is in PATH
    if command -v mcp-server &> /dev/null; then
        MCP_SERVER_PATH=$(which mcp-server)
        log_success "Found mcp-server: $MCP_SERVER_PATH"
        return 0
    fi
    
    # Check project target directories
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
    
    # Default to expecting it in PATH
    MCP_SERVER_PATH="mcp-server"
    log_warn "mcp-server not found, using 'mcp-server' (must be in PATH)"
    return 0
}

generate_mcp_config() {
    local mode="${1:-http}"
    
    if [[ "$mode" == "stdio" ]]; then
        # STDIO mode - direct process communication
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
    else
        # HTTP mode - SSE transport
        cat <<EOF
{
  "url": "http://$MCP_SERVER_HOST:$MCP_SERVER_PORT/sse",
  "transport": "sse"
}
EOF
    fi
}

update_claude_config() {
    local config_file="$1"
    local scope="$2"
    local mode="${3:-http}"
    
    log_info "Updating $scope config: $config_file"
    
    # Create backup if file exists
    if [[ -f "$config_file" ]]; then
        cp "$config_file" "${config_file}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing config"
    fi
    
    # Generate the MCP server config
    local mcp_config
    mcp_config=$(generate_mcp_config "$mode")
    
    # Create or update config file
    if [[ -f "$config_file" ]]; then
        # File exists - merge configuration
        local existing
        existing=$(cat "$config_file")
        
        # Check if mcpServers key exists
        if echo "$existing" | jq -e '.mcpServers' &> /dev/null; then
            # Add/update our server entry
            echo "$existing" | jq --argjson config "$mcp_config" \
                ".mcpServers[\"$MCP_SERVER_NAME\"] = \$config" > "$config_file"
        else
            # Add mcpServers object
            echo "$existing" | jq --argjson config "$mcp_config" \
                ". + {mcpServers: {\"$MCP_SERVER_NAME\": \$config}}" > "$config_file"
        fi
    else
        # Create new config file
        local new_config
        new_config=$(jq -n --argjson config "$mcp_config" \
            "{mcpServers: {\"$MCP_SERVER_NAME\": \$config}}")
        echo "$new_config" > "$config_file"
    fi
    
    log_success "Updated $config_file"
}

setup_user_scope() {
    log_info "Setting up user-scope integration..."
    update_claude_config "$CLAUDE_CONFIG_USER" "user" "$MODE"
}

setup_project_scope() {
    local project_dir="${1:-.}"
    local config_file="$project_dir/$CLAUDE_CONFIG_PROJECT"
    
    log_info "Setting up project-scope integration in $project_dir..."
    update_claude_config "$config_file" "project" "$MODE"
}

verify_installation() {
    log_info "Verifying installation..."
    
    # Check if config file exists and is valid JSON
    if [[ -f "$CLAUDE_CONFIG_USER" ]]; then
        if jq -e ".mcpServers[\"$MCP_SERVER_NAME\"]" "$CLAUDE_CONFIG_USER" &> /dev/null; then
            log_success "User config verified: $CLAUDE_CONFIG_USER"
        else
            log_warn "MCP server not found in user config"
        fi
    fi
    
    # Check if server is running
    if curl -s "http://$MCP_SERVER_HOST:$MCP_SERVER_PORT/api/health" &> /dev/null; then
        log_success "MCP Agent Mail server is running on port $MCP_SERVER_PORT"
    else
        log_warn "MCP Agent Mail server not responding on port $MCP_SERVER_PORT"
        log_info "Start it with: cargo run -p mcp-server"
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
    echo "  • Mode: $MODE"
    echo "  • Port: $MCP_SERVER_PORT"
    echo ""
    echo "Config files updated:"
    [[ -f "$CLAUDE_CONFIG_USER" ]] && echo "  • $CLAUDE_CONFIG_USER"
    [[ "$SCOPE" == "project" ]] && echo "  • $CLAUDE_CONFIG_PROJECT"
    echo ""
    echo "Next steps:"
    echo "  1. Restart Claude Code to load the new configuration"
    echo "  2. Ensure mcp-server is running: cargo run -p mcp-server"
    echo "  3. In Claude Code, the MCP tools should now be available"
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure Claude Code to use MCP Agent Mail.

Options:
  -m, --mode MODE       Connection mode: http (default) or stdio
  -s, --scope SCOPE     Config scope: user (default) or project
  -p, --project DIR     Project directory for project-scope config
  -P, --port PORT       MCP server port (default: 8765)
  -h, --help            Show this help message

Examples:
  $(basename "$0")                           # User scope, HTTP mode
  $(basename "$0") --mode stdio              # User scope, STDIO mode
  $(basename "$0") --scope project           # Project scope in current dir
  $(basename "$0") --project /path/to/proj   # Project scope in specific dir

Environment Variables:
  MCP_AGENT_MAIL_PORT   Server port (default: 8765)
  MCP_AGENT_MAIL_HOST   Server host (default: 127.0.0.1)
EOF
}

# Parse arguments
MODE="http"
SCOPE="user"
PROJECT_DIR="."

while [[ $# -gt 0 ]]; do
    case $1 in
        -m|--mode)
            MODE="$2"
            shift 2
            ;;
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

# Validate mode
if [[ "$MODE" != "http" && "$MODE" != "stdio" ]]; then
    log_error "Invalid mode: $MODE (must be 'http' or 'stdio')"
    exit 1
fi

# Main execution
main() {
    print_header
    check_dependencies
    detect_claude_code
    find_mcp_server
    
    if [[ "$SCOPE" == "project" ]]; then
        setup_project_scope "$PROJECT_DIR"
    else
        setup_user_scope
    fi
    
    verify_installation
    print_summary
}

main "$@"
