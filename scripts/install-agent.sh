#!/bin/bash
set -e

# Exit immediately if not running as root
if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (sudo)." >&2
  exit 1
fi

UNATTENDED=false
if [[ "$1" == "--unattended" ]]; then
  UNATTENDED=true
fi

echo "============================================="
echo "Installing TTGTiSO-Desk Remote Desktop Agent"
echo "============================================="

# Detect OS and install dependencies
if [ -f /etc/astra_version ]; then
  echo "Detected Astra Linux. Installing Debian/Astra dependencies..."
  apt-get update && apt-get install -y xvfb fly-wm xauth x11-utils socat netcat-openbsd libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools gstreamer1.0-x gstreamer1.0-alsa gstreamer1.0-gl gstreamer1.0-gtk3 gstreamer1.0-qt5 gstreamer1.0-pulseaudio xterm || true
elif [ -f /etc/altlinux-release ]; then
  echo "Detected Alt Linux. Installing Alt dependencies..."
  apt-get update && apt-get install -y xvfb openbox xauth x11-utils socat netcat libgstreamer1.0-devel gst-plugins-base1.0-devel gst-plugins-bad1.0-devel gst-plugins-good1.0 gst-plugins-bad1.0 gst-plugins-ugly1.0 gstreamer1.0-utils xterm || true
elif [ -f /etc/arch-release ]; then
  echo "Detected Arch Linux. Installing Arch dependencies..."
  pacman -S --noconfirm xorg-server-xvfb openbox xorg-xauth xorg-xinit socat gnu-netcat gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav xterm || true
else
  echo "Unknown OS. Checking package managers..."
  if command -v apt-get &> /dev/null; then
    apt-get update && apt-get install -y xvfb openbox xauth x11-utils socat netcat-openbsd gstreamer1.0-tools gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly xterm || true
  elif command -v pacman &> /dev/null; then
    pacman -S --noconfirm xorg-server-xvfb openbox xorg-xauth socat gnu-netcat gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav xterm || true
  fi
fi


# Define target directories
BIN_PATH="/usr/bin/ttgtiso-desk-agent"
CONF_DIR="/etc/ttgtiso-desk"
DATA_DIR="/var/lib/ttgtiso-desk"
LOG_DIR="/var/log/ttgtiso-desk"
SYSTEMD_UNIT="/etc/systemd/system/ttgtiso-desk-agent.service"

# Create directories
echo "Creating system directories..."
mkdir -p "$CONF_DIR"
mkdir -p "$DATA_DIR"
mkdir -p "$LOG_DIR"

chmod 755 "$DATA_DIR"
chmod 700 "$LOG_DIR"
chmod 750 "$CONF_DIR"

# Stop existing service if running to avoid "Text file busy"
echo "Stopping existing agent service if running..."
systemctl stop ttgtiso-desk-agent 2>/dev/null || true

# Copy binary (assuming compiled binary is in target/release/ or current folder)
if [ -f "./target/release/server-agent" ]; then
  echo "Copying compiled release binary..."
  cp "./target/release/server-agent" "$BIN_PATH"
elif [ -f "./server-agent" ]; then
  echo "Copying binary from current directory..."
  cp "./server-agent" "$BIN_PATH"
else
  echo "Warning: server-agent binary not found in release target or current dir."
  if [ "$UNATTENDED" = false ]; then
    read -p "Enter path to server-agent binary: " custom_bin_path
    if [ -f "$custom_bin_path" ]; then
      cp "$custom_bin_path" "$BIN_PATH"
    else
      echo "Error: Binary not found at $custom_bin_path" >&2
      exit 1
    fi
  else
    echo "Error: Unattended installation aborted - binary not found." >&2
    exit 1
  fi
fi

chmod 755 "$BIN_PATH"

# Copy default config if it doesn't exist
if [ ! -f "$CONF_DIR/agent.toml" ]; then
  echo "Installing default configuration..."
  if [ -f "./templates/agent.toml.default" ]; then
    cp "./templates/agent.toml.default" "$CONF_DIR/agent.toml"
  else
    cat <<EOF > "$CONF_DIR/agent.toml"
bind_address = "0.0.0.0"
port = 22
[session_limits]
max_concurrent_sessions = 5
session_timeout_seconds = 3600
[security_policy]
allow_password_auth = true
enable_audit_logs = true
EOF
  fi
  chmod 600 "$CONF_DIR/agent.toml"
fi

# Copy systemd unit file
echo "Installing Systemd unit..."
if [ -f "./templates/ttgtiso-desk-agent.service" ]; then
  cp "./templates/ttgtiso-desk-agent.service" "$SYSTEMD_UNIT"
else
  cat <<EOF > "$SYSTEMD_UNIT"
[Unit]
Description=TTGTiSO-Desk Remote Desktop Server Agent
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/var/lib/ttgtiso-desk
ExecStart=/usr/bin/ttgtiso-desk-agent --config /etc/ttgtiso-desk/agent.toml
Restart=always

[Install]
WantedBy=multi-user.target
EOF
fi

chmod 644 "$SYSTEMD_UNIT"

# Reload systemd and start service
echo "Starting and enabling service..."
systemctl daemon-reload
systemctl enable ttgtiso-desk-agent
systemctl start ttgtiso-desk-agent

echo "============================================="
echo "Installation completed successfully!"
echo "Service status:"
systemctl status ttgtiso-desk-agent --no-pager | grep -E "Active:" || true
echo "============================================="
