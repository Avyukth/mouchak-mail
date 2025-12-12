#!/usr/bin/env bash
# integrate_github_copilot.sh - Configure GitHub Copilot (VSCode) to use MCP Agent Mail
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

# GitHub Copilot uses VSCode's MCP config
# Location: ~/.vscode/mcp.json or .vscode/mcp.json in project
COPILOT_CONFIG_GLOBAL="$HOME/.vscode/mcp.json"
COPILOT_CONFIG_PROJECT=".vscode/mcp.json"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     MCP Agent Mail - GitHub Copilot Integration           ${BLUE}║${NC}"
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

detect_copilot() {
    log_info "Detecting GitHub Copilot extension..."
    
    # Check for Copilot extension in VSCode
    local copilot_paths=(
        "$HOME/.vscode/extensions/github.copilot-"*
        "$HOME/.vscode-server/extensions/github.copilot-"*
    )
    
    for path in "${copilot_paths[@]}"; do
        if [[ -d "$path" ]]; then
            log_success "Found GitHub Copilot extension"
            return 0
        fi
    done
    
    log_warn "GitHub Copilot extension not detected in VSCode"
    log_info "Install from: https://marketplace.visualstudio.com/items?itemName=GitHub.copilot"
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

update_copilot_config() {
    local config_file="$1"
    local scope="$2"
    
    log_info "Updating $scope config: $config_file"
    
    mkdir -p "$(dirname "$config_file")"
    
    if [[ -f "$config_file" ]]; then
        cp "$config_file" "${config_file}.backup.$(date +%Y%m%d%H%M%S)"
        log_info "Created backup of existing config"
    fi
    
    local mcp_config
    mcp_config=$(generate_mcp_config)
    
    if [[ -f "$config_file" ]]; then
        local existing
        existing=$(cat "$config_file")
        
        if echo "$existing" | jq -e '.servers' &> /dev/null; then
            echo "$existing" | jq --argjson config "$mcp_config" \
                ".servers[\"$MCP_SERVER_NAME\"] = \$config" > "$config_file"
        else
            echo "$existing" | jq --argjson config "$mcp_config" \
                ". + {servers: {\"$MCP_SERVER_NAME\": \$config}}" > "$config_file"
        fi
    else
        jq -n --argjson config "$mcp_config" \
            "{servers: {\"$MCP_SERVER_NAME\": \$config}}" > "$config_file"
    fi
    
    log_success "Updated $config_file"
}

setup_global_scope() {
    log_info "Setting up global VSCode MCP config..."
    update_copilot_config "$COPILOT_CONFIG_GLOBAL" "global"
}

setup_project_scope() {
    local project_dir="${1:-.}"
    local config_file="$project_dir/$COPILOT_CONFIG_PROJECT"
    
    log_info "Setting up project-scope integration in $project_dir..."
    update_copilot_config "$config_file" "project"
}

verify_installation() {
    log_info "Verifying installation..."
    
    if [[ -f "$COPILOT_CONFIG_GLOBAL" ]]; then
        if jq -e ".servers[\"$MCP_SERVER_NAME\"]" "$COPILOT_CONFIG_GLOBAL" &> /dev/null; then
            log_success "Global config verified: $COPILOT_CONFIG_GLOBAL"
        else
            log_warn "MCP server not found in global config"
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
    echo ""
    echo "Next steps:"
    echo "  1. Reload VSCode window"
    echo "  2. Copilot Agent should now have access to MCP tools"
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Configure GitHub Copilot (VSCode) to use MCP Agent Mail.

Options:
  -s, --scope SCOPE     Config scope: global (default) or project
  -p, --project DIR     Project directory for project-scope config
  -P, --port PORT       MCP server port (default: 8765)
  -h, --help            Show this help message
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
    detect_copilot
    find_mcp_server
    
    if [[ "$SCOPE" == "project" ]]; then
        setup_project_scope "$PROJECT_DIR"
    else
        setup_global_scope
    fi
    
    verify_installation
    print_summary
}

main "$@"
