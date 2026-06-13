#!/bin/bash
set -e

# Ensure the script is run with sudo/root privileges
if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run this script with sudo:"
  echo "sudo $0"
  exit 1
fi

# Retrieve the actual non-root username who invoked sudo
REAL_USER=$SUDO_USER
if [ -z "$REAL_USER" ] || [ "$REAL_USER" = "root" ]; then
  REAL_USER=$(logname 2>/dev/null || echo $USER)
fi

echo "=========================================================="
echo "   TTGTiSO-Desk Agent Automated Setup for Arch Linux   "
echo "=========================================================="
echo "Invoked by system user: $REAL_USER"
echo ""

# 1. Update pacman and install system dependencies (including openssh)
echo "📦 Step 1: Installing system dependencies and OpenSSH..."
pacman -Sy --noconfirm \
  openssh \
  xorg-server-xvfb \
  openbox \
  xorg-xauth \
  socat \
  openbsd-netcat \
  gstreamer \
  gst-plugins-base \
  gst-plugins-good \
  gst-plugins-bad \
  gst-plugins-ugly \
  gst-libav \
  xterm \
  || { echo "Failed to install packages via pacman." >&2; exit 1; }

# 2. Configure and start SSH daemon
echo ""
echo "🔌 Step 2: Enabling and starting SSH daemon (sshd)..."
systemctl enable --now sshd
if systemctl is-active --quiet sshd; then
  echo "✅ SSH daemon (sshd) is running on port 22."
else
  echo "❌ Failed to start sshd. Please check systemctl status sshd." >&2
  exit 1
fi

# 3. Setup Agent configuration directories and config file
echo ""
echo "⚙️ Step 3: Configuring Agent to run on port 2222 (avoiding SSH conflict)..."
CONF_DIR="/etc/ttgtiso-desk"
DATA_DIR="/var/lib/ttgtiso-desk"
LOG_DIR="/var/log/ttgtiso-desk"
BIN_PATH="/usr/bin/ttgtiso-desk-agent"
SYSTEMD_UNIT="/etc/systemd/system/ttgtiso-desk-agent.service"

mkdir -p "$CONF_DIR"
mkdir -p "$DATA_DIR"
mkdir -p "$LOG_DIR"

# Write custom agent configuration
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

chmod 750 "$CONF_DIR"
chmod 600 "$CONF_DIR/agent.toml"
chmod 755 "$DATA_DIR"
chmod 700 "$LOG_DIR"
echo "✅ Agent configured successfully. Config written to $CONF_DIR/agent.toml"

# 4. Authorize main server's SSH public key
echo ""
echo "🔑 Step 4: Setting up passwordless SSH access..."
USER_HOME=$(eval echo "~$REAL_USER")
SSH_DIR="$USER_HOME/.ssh"

# Ensure .ssh directory exists with correct permissions
sudo -u "$REAL_USER" mkdir -p "$SSH_DIR"
sudo -u "$REAL_USER" chmod 700 "$SSH_DIR"

echo "Please paste the SSH public key (id_rsa.pub) of the main server (from 192.168.1.47),"
echo "then press Enter:"
read -r PUBLIC_KEY

if [ -n "$PUBLIC_KEY" ]; then
  AUTH_KEYS="$SSH_DIR/authorized_keys"
  echo "$PUBLIC_KEY" | sudo -u "$REAL_USER" tee -a "$AUTH_KEYS" >/dev/null
  sudo -u "$REAL_USER" chmod 600 "$AUTH_KEYS"
  echo "✅ Public key successfully appended to $AUTH_KEYS."
else
  echo "⚠️ No key entered. You will need to add it manually later to enable connection."
fi

# 5. Compile and install the server-agent binary
echo ""
echo "🛠️ Step 5: Compiling and installing the agent binary..."
if [ -f "./target/release/server-agent" ]; then
  echo "Using already compiled release binary..."
  cp "./target/release/server-agent" "$BIN_PATH"
elif [ -f "./server-agent" ]; then
  echo "Using binary from current directory..."
  cp "./server-agent" "$BIN_PATH"
else
  echo "Binary not pre-compiled. Initiating cargo build as $REAL_USER..."
  sudo -u "$REAL_USER" cargo build --release --bin server-agent
  if [ -f "./target/release/server-agent" ]; then
    cp "./target/release/server-agent" "$BIN_PATH"
  else
    echo "❌ Compilation failed. Ensure Rust and Cargo are installed." >&2
    exit 1
  fi
fi

chmod 755 "$BIN_PATH"
echo "✅ Binary installed to $BIN_PATH."

# 6. Install and start the systemd unit service
echo ""
echo "🚀 Step 6: Installing and starting systemd service..."
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

chmod 644 "$SYSTEMD_UNIT"
systemctl daemon-reload
systemctl enable ttgtiso-desk-agent
systemctl restart ttgtiso-desk-agent

echo ""
echo "=========================================================="
echo "🎉 Setup completed successfully!"
echo "=========================================================="
echo "1. SSH Daemon is running on port 22."
echo "2. TTGTiSO-Desk Agent is running on port 2222."
echo "3. Active sessions can be queried via Unix socket."
echo ""
echo "Next Steps:"
echo "Register this host on the main server (192.168.1.47) with:"
echo " - IP: $(ip route get 1 | awk '{print $7;exit}')"
echo " - SSH Port: 22"
echo " - Username: $REAL_USER"
echo "=========================================================="
