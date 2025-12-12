#!/bin/bash
set -e

# Configuration
SERVICE_NAME="mcp-server"
BINARY_PATH="./target/release/mcp-server" # Assumes running from repo root
INSTALL_BIN="/usr/local/bin/mcp-server"
INSTALL_SERVICE="/etc/systemd/system/${SERVICE_NAME}.service"
CONFIG_DIR="/etc/${SERVICE_NAME}"
DATA_DIR="/var/lib/${SERVICE_NAME}"
USER_NAME="mcp"

echo " Installing ${SERVICE_NAME}..."

# Check root
if [ "$EUID" -ne 0 ]; then
  echo "Please run as root"
  exit 1
fi

# Create User
if id "$USER_NAME" &>/dev/null; then
    echo "User ${USER_NAME} already exists"
else
    echo "Creating user ${USER_NAME}"
    useradd -r -s /bin/false ${USER_NAME}
fi

# directories
echo "Creating directories..."
mkdir -p ${CONFIG_DIR}
mkdir -p ${DATA_DIR}
chown -R ${USER_NAME}:${USER_NAME} ${CONFIG_DIR}
chown -R ${USER_NAME}:${USER_NAME} ${DATA_DIR}

# Copy Binary
echo "Copying binary..."
if [ -f "$BINARY_PATH" ]; then
    cp "$BINARY_PATH" "$INSTALL_BIN"
    chmod +x "$INSTALL_BIN"
else
    echo "Warning: Binary not found at $BINARY_PATH. Skipping copy."
fi

# Copy Service
echo "Copying service file..."
cp deploy/systemd/${SERVICE_NAME}.service ${INSTALL_SERVICE}

# Copy Env (if not exists)
if [ ! -f "${CONFIG_DIR}/env" ]; then
    echo "Copying example env..."
    cp deploy/systemd/${SERVICE_NAME}.env.example ${CONFIG_DIR}/env
    echo "Please edit ${CONFIG_DIR}/env with your configuration."
fi
chown ${USER_NAME}:${USER_NAME} ${CONFIG_DIR}/env
chmod 600 ${CONFIG_DIR}/env

# Reload
echo "Reloading systemd..."
if command -v systemctl &> /dev/null; then
    systemctl daemon-reload
    echo "Service installed. To start:"
    echo "  systemctl enable --now ${SERVICE_NAME}"
    echo "  systemctl status ${SERVICE_NAME}"
else
    echo "systemctl not found. Is this a systemd system?"
fi

echo "Done."
