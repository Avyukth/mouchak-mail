#!/bin/bash
set -e

# Configuration
BINARY_PATH="./target/release/mouchak-mail"
INSTALL_PATH="/usr/local/bin/mouchak-mail"
USER="mouchak"
SERVICE_NAME="mouchak-mail"
DATA_DIR="/var/lib/mouchak-mail"

# Check root
if [ "$EUID" -ne 0 ]; then 
  echo "Please run as root"
  exit 1
fi

echo "Installing $SERVICE_NAME..."

# Create user
if ! id "$USER" &>/dev/null; then
    useradd -r -s /bin/false $USER
    echo "Created user $USER"
fi

# Create data directory
mkdir -p $DATA_DIR
chown -R $USER:$USER $DATA_DIR
echo "Created Data Directory $DATA_DIR"

# Install Binary
if [ -f "$BINARY_PATH" ]; then
    cp "$BINARY_PATH" "$INSTALL_PATH"
    chmod +x "$INSTALL_PATH"
    echo "Installed binary to $INSTALL_PATH"
else
    echo "Binary not found at $BINARY_PATH. Please build release first."
    exit 1
fi

# Install Service
cp deploy/systemd/$SERVICE_NAME.service /etc/systemd/system/
systemctl daemon-reload
echo "Installed systemd service"

# Enable
systemctl enable $SERVICE_NAME
echo "Enabled service $SERVICE_NAME"
echo "Installation complete. Configure /etc/default/$SERVICE_NAME and start service."
