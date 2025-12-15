#!/bin/bash
# MCP Agent Mail Installer
# One-liner: curl -fsSL https://raw.githubusercontent.com/mouchak/mcp-agent-mail-rs/main/scripts/install.sh | bash
#
# This script:
# 1. Detects OS and architecture
# 2. Downloads the appropriate binary from GitHub releases
# 3. Installs to ~/.local/bin
# 4. Creates launchd plist (macOS) or systemd user unit (Linux)
# 5. Starts the server
# 6. Verifies health

set -e

# Configuration
REPO="mouchak/mcp-agent-mail-rs"
BIN_NAME="mcp-agent-mail"
INSTALL_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/.local/share/mcp-agent-mail"
CONFIG_DIR="${HOME}/.config/mcp-agent-mail"
DEFAULT_PORT=8765

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Darwin) echo "darwin" ;;
        Linux) echo "linux" ;;
        *) error "Unsupported OS: $(uname -s)" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "amd64" ;;
        aarch64|arm64) echo "arm64" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Get latest release version from GitHub
get_latest_version() {
    if command -v curl &>/dev/null; then
        curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | \
            grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/' || echo ""
    elif command -v wget &>/dev/null; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | \
            grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/' || echo ""
    fi
}

# Download file
download() {
    local url="$1"
    local dest="$2"

    if command -v curl &>/dev/null; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget &>/dev/null; then
        wget -q "$url" -O "$dest"
    else
        error "Neither curl nor wget found. Please install one."
    fi
}

# Create directories
create_dirs() {
    info "Creating directories..."
    mkdir -p "${INSTALL_DIR}"
    mkdir -p "${DATA_DIR}"
    mkdir -p "${CONFIG_DIR}"
    success "Directories created"
}

# Download and install binary
install_binary() {
    local os="$1"
    local arch="$2"
    local version="$3"

    # If no release version, try to build from source or use local
    if [ -z "$version" ]; then
        warn "No release found. Checking for local binary..."

        # Check if running from repo with built binary
        local local_binary="./target/release/${BIN_NAME}"
        if [ -f "$local_binary" ]; then
            info "Found local binary at ${local_binary}"
            cp "$local_binary" "${INSTALL_DIR}/${BIN_NAME}"
            chmod +x "${INSTALL_DIR}/${BIN_NAME}"
            success "Installed local binary to ${INSTALL_DIR}/${BIN_NAME}"
            return 0
        fi

        # Try cargo install
        if command -v cargo &>/dev/null; then
            info "Building from source with cargo..."
            cargo install --path . --root "${HOME}/.local" 2>/dev/null || \
            cargo install --git "https://github.com/${REPO}" --root "${HOME}/.local" 2>/dev/null || \
            error "Failed to build from source"
            success "Built and installed from source"
            return 0
        fi

        error "No release available and cargo not found. Please build manually or wait for a release."
    fi

    # Download from release
    local filename="${BIN_NAME}-${version}-${os}-${arch}"
    [ "$os" = "darwin" ] && filename="${filename}.tar.gz" || filename="${filename}.tar.gz"

    local url="https://github.com/${REPO}/releases/download/${version}/${filename}"
    local tmp_dir=$(mktemp -d)
    local tmp_file="${tmp_dir}/${filename}"

    info "Downloading ${BIN_NAME} ${version} for ${os}/${arch}..."
    download "$url" "$tmp_file" || error "Download failed from ${url}"

    info "Extracting..."
    tar -xzf "$tmp_file" -C "$tmp_dir"

    local extracted_bin="${tmp_dir}/${BIN_NAME}"
    [ ! -f "$extracted_bin" ] && extracted_bin=$(find "$tmp_dir" -name "$BIN_NAME" -type f | head -1)

    if [ -f "$extracted_bin" ]; then
        mv "$extracted_bin" "${INSTALL_DIR}/${BIN_NAME}"
        chmod +x "${INSTALL_DIR}/${BIN_NAME}"
        success "Installed to ${INSTALL_DIR}/${BIN_NAME}"
    else
        error "Binary not found in archive"
    fi

    rm -rf "$tmp_dir"
}

# Create macOS launchd plist
create_launchd_plist() {
    local plist_dir="${HOME}/Library/LaunchAgents"
    local plist_file="${plist_dir}/com.mouchak.mcp-agent-mail.plist"

    mkdir -p "$plist_dir"

    info "Creating launchd plist..."
    cat > "$plist_file" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mouchak.mcp-agent-mail</string>
    <key>ProgramArguments</key>
    <array>
        <string>${INSTALL_DIR}/${BIN_NAME}</string>
        <string>serve</string>
        <string>http</string>
        <string>--port</string>
        <string>${DEFAULT_PORT}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${DATA_DIR}/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>${DATA_DIR}/stderr.log</string>
    <key>WorkingDirectory</key>
    <string>${DATA_DIR}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:${INSTALL_DIR}</string>
    </dict>
</dict>
</plist>
EOF

    success "Created launchd plist at ${plist_file}"
    echo "$plist_file"
}

# Create Linux systemd user unit
create_systemd_unit() {
    local unit_dir="${HOME}/.config/systemd/user"
    local unit_file="${unit_dir}/mcp-agent-mail.service"

    mkdir -p "$unit_dir"

    info "Creating systemd user unit..."
    cat > "$unit_file" << EOF
[Unit]
Description=MCP Agent Mail Server
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/${BIN_NAME} serve http --port ${DEFAULT_PORT}
WorkingDirectory=${DATA_DIR}
Restart=always
RestartSec=5
StandardOutput=append:${DATA_DIR}/stdout.log
StandardError=append:${DATA_DIR}/stderr.log

[Install]
WantedBy=default.target
EOF

    success "Created systemd unit at ${unit_file}"
    echo "$unit_file"
}

# Start service
start_service() {
    local os="$1"

    info "Starting service..."

    if [ "$os" = "darwin" ]; then
        launchctl unload ~/Library/LaunchAgents/com.mouchak.mcp-agent-mail.plist 2>/dev/null || true
        launchctl load ~/Library/LaunchAgents/com.mouchak.mcp-agent-mail.plist
        success "Service started via launchd"
    else
        systemctl --user daemon-reload
        systemctl --user enable mcp-agent-mail.service
        systemctl --user restart mcp-agent-mail.service
        success "Service started via systemd"
    fi
}

# Verify health
verify_health() {
    local max_attempts=10
    local attempt=1

    info "Waiting for server to start..."
    sleep 2

    while [ $attempt -le $max_attempts ]; do
        if curl -s "http://127.0.0.1:${DEFAULT_PORT}/health" &>/dev/null; then
            local health=$(curl -s "http://127.0.0.1:${DEFAULT_PORT}/health")
            success "Server is healthy!"
            echo "  Response: ${health}"
            return 0
        fi

        info "Attempt ${attempt}/${max_attempts}..."
        sleep 1
        ((attempt++))
    done

    warn "Server health check failed after ${max_attempts} attempts"
    warn "Check logs at ${DATA_DIR}/stderr.log"
    return 1
}

# Add to PATH
add_to_path() {
    local shell_rc=""

    case "$SHELL" in
        */zsh) shell_rc="${HOME}/.zshrc" ;;
        */bash) shell_rc="${HOME}/.bashrc" ;;
        *) shell_rc="${HOME}/.profile" ;;
    esac

    if [ -f "$shell_rc" ]; then
        if ! grep -q "${INSTALL_DIR}" "$shell_rc" 2>/dev/null; then
            echo "" >> "$shell_rc"
            echo "# MCP Agent Mail" >> "$shell_rc"
            echo "export PATH=\"\${PATH}:${INSTALL_DIR}\"" >> "$shell_rc"
            info "Added ${INSTALL_DIR} to PATH in ${shell_rc}"
        fi
    fi
}

# Print usage instructions
print_usage() {
    echo ""
    echo -e "${GREEN}Installation complete!${NC}"
    echo ""
    echo "Binary location: ${INSTALL_DIR}/${BIN_NAME}"
    echo "Data directory:  ${DATA_DIR}"
    echo "Config directory: ${CONFIG_DIR}"
    echo ""
    echo "Commands:"
    echo "  ${BIN_NAME} serve http --port 8765  # Start HTTP server"
    echo "  ${BIN_NAME} serve mcp               # Start MCP stdio server"
    echo "  ${BIN_NAME} health                  # Check server health"
    echo "  ${BIN_NAME} version                 # Show version"
    echo ""
    echo "Server is running at: http://127.0.0.1:${DEFAULT_PORT}"
    echo "Health endpoint:      http://127.0.0.1:${DEFAULT_PORT}/health"
    echo ""
    echo "To stop the service:"
    if [ "$(detect_os)" = "darwin" ]; then
        echo "  launchctl unload ~/Library/LaunchAgents/com.mouchak.mcp-agent-mail.plist"
    else
        echo "  systemctl --user stop mcp-agent-mail"
    fi
    echo ""
    echo "Logs: ${DATA_DIR}/stderr.log"
}

# Main installation
main() {
    echo ""
    echo -e "${BLUE}╔══════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║     MCP Agent Mail Installer             ║${NC}"
    echo -e "${BLUE}╚══════════════════════════════════════════╝${NC}"
    echo ""

    local os=$(detect_os)
    local arch=$(detect_arch)

    info "Detected: ${os}/${arch}"

    # Get version
    local version=$(get_latest_version)
    if [ -n "$version" ]; then
        info "Latest release: ${version}"
    else
        warn "No GitHub release found"
    fi

    # Create directories
    create_dirs

    # Install binary
    install_binary "$os" "$arch" "$version"

    # Add to PATH
    add_to_path

    # Create service files
    if [ "$os" = "darwin" ]; then
        create_launchd_plist
    else
        create_systemd_unit
    fi

    # Start service
    start_service "$os"

    # Verify health
    verify_health || true

    # Print usage
    print_usage
}

# Run main
main "$@"
