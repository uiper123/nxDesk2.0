#!/bin/bash
set -e

# Exit immediately if not running as root
if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (sudo)." >&2
  exit 1
fi

echo "============================================="
echo "Uninstalling TTGTiSO-Desk Remote Desktop Agent"
echo "============================================="

# Stop and disable systemd service
if systemctl is-active --quiet ttgtiso-desk-agent; then
  echo "Stopping service..."
  systemctl stop ttgtiso-desk-agent
fi

if systemctl is-enabled --quiet ttgtiso-desk-agent; then
  echo "Disabling service..."
  systemctl disable ttgtiso-desk-agent
fi

# Remove systemd unit file
if [ -f "/etc/systemd/system/ttgtiso-desk-agent.service" ]; then
  echo "Removing systemd unit..."
  rm -f "/etc/systemd/system/ttgtiso-desk-agent.service"
  systemctl daemon-reload
fi

# Remove binary
if [ -f "/usr/bin/ttgtiso-desk-agent" ]; then
  echo "Removing binary..."
  rm -f "/usr/bin/ttgtiso-desk-agent"
fi

# Prompt or remove config/data directories
if [[ "$1" == "--purge" ]]; then
  echo "Purging all configuration, data, and log directories..."
  rm -rf "/etc/ttgtiso-desk"
  rm -rf "/var/lib/ttgtiso-desk"
  rm -rf "/var/log/ttgtiso-desk"
else
  echo "Keeping configuration, data and log directories (/etc/ttgtiso-desk, /var/lib/ttgtiso-desk, /var/log/ttgtiso-desk)."
  echo "To remove them, run: sudo $0 --purge"
fi

echo "============================================="
echo "Uninstallation completed successfully!"
echo "============================================="
