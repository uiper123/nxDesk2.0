#!/bin/bash
set -e

# Exit immediately if not running as root
if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (sudo)." >&2
  exit 1
fi

UNATTENDED=false
FROM_GITHUB=false
REPO="${TTGTISO_REPO:-uiper123/nxDesk2.0}"
PUBLIC_KEY=""

# Parse command line options
for arg in "$@"; do
  case "$arg" in
    --unattended) UNATTENDED=true ;;
    --from-github) FROM_GITHUB=true ;;
    *)
      if [[ "$arg" =~ ^ssh- ]]; then
        PUBLIC_KEY="$arg"
      fi
      ;;
  esac
done

echo "============================================="
echo "Installing TTGTiSO-Desk Remote Desktop Agent"
echo "============================================="

# Detect OS and install dependencies
if [ -f /etc/astra_version ]; then
  echo "Detected Astra Linux. Installing Astra dependencies..."
  apt-get update && apt-get install -y openssh-server xvfb fly-wm xauth x11-utils socat netcat-openbsd libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools gstreamer1.0-x gstreamer1.0-alsa gstreamer1.0-gl gstreamer1.0-gtk3 gstreamer1.0-pulseaudio xterm || true
elif [ -f /etc/altlinux-release ]; then
  echo "Detected Alt Linux. Installing Alt dependencies..."
  apt-get update && apt-get install -y openssh-server xvfb openbox xauth x11-utils socat netcat-openbsd gst-plugins-base1.0 gst-plugins-good1.0 gst-plugins-bad1.0 gst-plugins-ugly1.0 gst-libav1.0 gstreamer1.0-utils xterm || true
elif [ -f /etc/arch-release ] || [ -f /etc/manjaro-release ]; then
  echo "Detected Arch/Manjaro Linux. Installing Arch dependencies..."
  pacman -Sy --noconfirm openssh xorg-server-xvfb openbox xorg-xauth socat openbsd-netcat gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav xterm || true
elif [ -f /etc/redhat-release ] || [ -f /etc/fedora-release ] || [ -f /etc/rocky-release ]; then
  echo "Detected RedHat/Fedora/Rocky Linux. Installing dependencies..."
  dnf install -y openssh-server xorg-x11-server-Xvfb openbox xorg-x11-xauth socat nmap-ncat gstreamer1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-plugins-ugly-free gstreamer1-libav xterm || true
else
  echo "Unknown OS. Checking package managers..."
  if command -v apt-get &> /dev/null; then
    apt-get update && apt-get install -y openssh-server xvfb openbox xauth x11-utils socat netcat-openbsd gstreamer1.0-tools gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly xterm || true
  elif command -v pacman &> /dev/null; then
    pacman -Sy --noconfirm openssh xorg-server-xvfb openbox xorg-xauth socat openbsd-netcat gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav xterm || true
  elif command -v dnf &> /dev/null; then
    dnf install -y openssh-server xorg-x11-server-Xvfb openbox xorg-x11-xauth socat nmap-ncat gstreamer1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-plugins-ugly-free gstreamer1-libav xterm || true
  fi
fi

# Enable and start SSH service
echo "🔌 Enabling and starting SSH daemon..."
if command -v systemctl &> /dev/null; then
  systemctl enable sshd 2>/dev/null || systemctl enable ssh 2>/dev/null || true
  systemctl start sshd 2>/dev/null || systemctl start ssh 2>/dev/null || true
else
  service sshd start 2>/dev/null || service ssh start 2>/dev/null || true
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

chown -R root:root "$DATA_DIR" "$LOG_DIR" "$CONF_DIR"

chmod 755 "$DATA_DIR"
chmod 700 "$LOG_DIR"
chmod 750 "$CONF_DIR"

# Stop existing service if running to avoid "Text file busy"
echo "Stopping existing agent service if running..."
systemctl stop ttgtiso-desk-agent 2>/dev/null || true

# Download binary from the latest GitHub release if requested
if [ "$FROM_GITHUB" = true ]; then
  echo "Downloading latest agent binary from GitHub releases (${REPO})...."
  TMP_BIN="$(mktemp /tmp/ttgtiso-agent.XXXXXX)"
  curl -fsSL "https://github.com/${REPO}/releases/latest/download/ttgtiso-desk-agent-linux-x86_64" -o "$TMP_BIN"
  TMP_SUMS="$(mktemp /tmp/ttgtiso-sums.XXXXXX)"
  if curl -fsSL "https://github.com/${REPO}/releases/latest/download/SHA256SUMS" -o "$TMP_SUMS"; then
    echo "Verifying checksum..."
    EXPECTED="$(grep ' ttgtiso-desk-agent-linux-x86_64$' "$TMP_SUMS" | awk '{print $1}')"
    ACTUAL="$(sha256sum "$TMP_BIN" | awk '{print $1}')"
    if [ -n "$EXPECTED" ] && [ "$EXPECTED" != "$ACTUAL" ]; then
      echo "Error: Checksum mismatch for downloaded binary." >&2
      rm -f "$TMP_BIN" "$TMP_SUMS"
      exit 1
    fi
  else
    echo "Warning: SHA256SUMS not found in release; skipping checksum verification." >&2
  fi
  cp "$TMP_BIN" "$BIN_PATH"
  rm -f "$TMP_BIN" "$TMP_SUMS"
# Copy binary (assuming compiled binary is in target/release/ or current folder)
elif [ -f "./target/release/server-agent" ]; then
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
port = 2222
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

# Authorize SSH public key
echo ""
echo "🔑 Setting up passwordless SSH access..."
REAL_USER=${SUDO_USER:-$USER}
USER_HOME=$(eval echo "~$REAL_USER")
SSH_DIR="$USER_HOME/.ssh"

# Ensure .ssh directory exists with correct permissions
sudo -u "$REAL_USER" mkdir -p "$SSH_DIR"
sudo -u "$REAL_USER" chmod 700 "$SSH_DIR"

if [ -z "$PUBLIC_KEY" ] && [ "$UNATTENDED" = false ]; then
  echo "Please paste the SSH public key (id_rsa.pub) of the main server,"
  echo "then press Enter (or press Enter to skip):"
  read -r PUBLIC_KEY
fi

if [ -n "$PUBLIC_KEY" ]; then
  AUTH_KEYS="$SSH_DIR/authorized_keys"
  # Clean up any bad inputs (e.g. command names if pasted by mistake)
  sed -i '/git push/d' "$AUTH_KEYS" 2>/dev/null || true
  
  echo "$PUBLIC_KEY" | sudo -u "$REAL_USER" tee -a "$AUTH_KEYS" >/dev/null
  sudo -u "$REAL_USER" chmod 600 "$AUTH_KEYS"
  echo "✅ Public key successfully appended to $AUTH_KEYS."
else
  echo "⚠️ Skipping SSH key setup."
fi

# Install the self-update helper if present
if [ -f "./scripts/update-agent.sh" ]; then
  echo "Installing update helper to /usr/bin/ttgtiso-desk-update..."
  install -m 755 "./scripts/update-agent.sh" /usr/bin/ttgtiso-desk-update
elif [ -f "./update-agent.sh" ]; then
  echo "Installing update helper to /usr/bin/ttgtiso-desk-update..."
  install -m 755 "./update-agent.sh" /usr/bin/ttgtiso-desk-update
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
systemctl restart ttgtiso-desk-agent

echo "============================================="
echo "Installation completed successfully!"
echo "Service status:"
systemctl status ttgtiso-desk-agent --no-pager | grep -E "Active:" || true
echo "============================================="
