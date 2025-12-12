#!/usr/bin/env bash
# integrate_all.sh - Auto-detect and configure all installed coding agents
# Part of mcp-agent-mail-rs integration scripts

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Track results
declare -a DETECTED=()
declare -a CONFIGURED=()
declare -a FAILED=()

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

print_header() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}  MCP Agent Mail - Auto-Detect & Configure All Agents      ${CYAN}║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════╝${NC}"
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
    echo ""
}

# Detection functions
detect_claude_code() {
    if command -v claude &> /dev/null || [[ -f "$HOME/.claude.json" ]]; then
        return 0
    fi
    return 1
}

detect_cursor() {
    if [[ -d "/Applications/Cursor.app" ]] || [[ -d "$HOME/Applications/Cursor.app" ]] || [[ -d "$HOME/.cursor" ]]; then
        return 0
    fi
    return 1
}

detect_windsurf() {
    if [[ -d "/Applications/Windsurf.app" ]] || [[ -d "$HOME/.codeium/windsurf" ]]; then
        return 0
    fi
    return 1
}

detect_cline() {
    for path in "$HOME/.vscode/extensions/saoudrizwan.claude-dev-"*; do
        if [[ -d "$path" ]]; then
            return 0
        fi
    done
    return 1
}

detect_codex() {
    if command -v codex &> /dev/null || [[ -d "$HOME/.codex" ]]; then
        return 0
    fi
    return 1
}

detect_gemini() {
    if command -v gemini &> /dev/null || [[ -d "$HOME/.gemini" ]]; then
        return 0
    fi
    return 1
}

detect_copilot() {
    for path in "$HOME/.vscode/extensions/github.copilot-"*; do
        if [[ -d "$path" ]]; then
            return 0
        fi
    done
    return 1
}

detect_opencode() {
    if command -v opencode &> /dev/null || [[ -d "$HOME/.opencode" ]]; then
        return 0
    fi
    return 1
}

detect_antigravity() {
    if [[ -d "$HOME/.gemini/antigravity" ]] || [[ -d "$HOME/.gemini" ]]; then
        return 0
    fi
    return 1
}

# Run detection and configuration
run_detection() {
    log_info "Scanning for installed coding agents..."
    echo ""
    
    local agents=(
        "claude_code:Claude Code"
        "cursor:Cursor IDE"
        "windsurf:Windsurf IDE"
        "cline:Cline (VSCode)"
        "codex:Codex CLI"
        "gemini:Gemini CLI"
        "copilot:GitHub Copilot"
        "opencode:OpenCode"
        "antigravity:Antigravity"
    )
    
    for agent_info in "${agents[@]}"; do
        local key="${agent_info%%:*}"
        local name="${agent_info#*:}"
        
        if "detect_$key" 2>/dev/null; then
            echo -e "  ${GREEN}✓${NC} $name detected"
            DETECTED+=("$key")
        else
            echo -e "  ${YELLOW}○${NC} $name not found"
        fi
    done
    
    echo ""
    
    if [[ ${#DETECTED[@]} -eq 0 ]]; then
        log_warn "No coding agents detected"
        echo "  You can still run individual integration scripts manually."
        return 1
    fi
    
    log_success "Detected ${#DETECTED[@]} coding agent(s)"
    return 0
}

run_configuration() {
    echo ""
    log_info "Configuring detected agents..."
    echo ""
    
    local script_map=(
        "claude_code:integrate_claude_code.sh"
        "cursor:integrate_cursor.sh"
        "windsurf:integrate_windsurf.sh"
        "cline:integrate_cline.sh"
        "codex:integrate_codex_cli.sh"
        "gemini:integrate_gemini_cli.sh"
        "copilot:integrate_github_copilot.sh"
        "opencode:integrate_opencode.sh"
        "antigravity:integrate_antigravity.sh"
    )
    
    for agent in "${DETECTED[@]}"; do
        for mapping in "${script_map[@]}"; do
            local key="${mapping%%:*}"
            local script="${mapping#*:}"
            
            if [[ "$key" == "$agent" ]]; then
                local script_path="$SCRIPT_DIR/$script"
                
                if [[ -x "$script_path" ]]; then
                    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
                    if "$script_path" "$@" 2>/dev/null; then
                        CONFIGURED+=("$agent")
                    else
                        FAILED+=("$agent")
                    fi
                else
                    log_warn "Script not found: $script_path"
                    FAILED+=("$agent")
                fi
                break
            fi
        done
    done
}

print_summary() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}                    Summary                                  ${CYAN}║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    echo "Detected: ${#DETECTED[@]} agent(s)"
    echo "Configured: ${#CONFIGURED[@]} agent(s)"
    
    if [[ ${#CONFIGURED[@]} -gt 0 ]]; then
        echo ""
        echo -e "${GREEN}Successfully configured:${NC}"
        for agent in "${CONFIGURED[@]}"; do
            echo "  • $agent"
        done
    fi
    
    if [[ ${#FAILED[@]} -gt 0 ]]; then
        echo ""
        echo -e "${RED}Failed to configure:${NC}"
        for agent in "${FAILED[@]}"; do
            echo "  • $agent"
        done
    fi
    
    echo ""
    echo "Next steps:"
    echo "  1. Start the MCP server: cargo run -p mcp-server"
    echo "  2. Restart your coding agents to load the new configuration"
    echo "  3. MCP Agent Mail tools should now be available"
    echo ""
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Auto-detect installed coding agents and configure them to use MCP Agent Mail.

Options:
  -P, --port PORT       MCP server port (default: 8765)
  -d, --detect-only     Only detect agents, don't configure
  -h, --help            Show this help message

Supported Agents:
  • Claude Code
  • Cursor IDE
  • Windsurf IDE
  • Cline (VSCode Extension)
  • Codex CLI
  • Gemini CLI
  • GitHub Copilot
  • OpenCode
  • Antigravity

Examples:
  $(basename "$0")                    # Detect and configure all
  $(basename "$0") --detect-only      # Only show detected agents
  $(basename "$0") --port 9000        # Use custom port for all
EOF
}

# Parse arguments
DETECT_ONLY=false
PASSTHROUGH_ARGS=()

while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--detect-only)
            DETECT_ONLY=true
            shift
            ;;
        -P|--port)
            PASSTHROUGH_ARGS+=("-P" "$2")
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
    
    if ! run_detection; then
        exit 0
    fi
    
    if [[ "$DETECT_ONLY" == true ]]; then
        echo ""
        log_info "Detection only mode - skipping configuration"
        exit 0
    fi
    
    run_configuration "${PASSTHROUGH_ARGS[@]}"
    print_summary
}

main "$@"
