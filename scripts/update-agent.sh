#!/bin/bash
# Self-update script for the TTGTiSO-Desk server agent.
# Downloads the latest release binary from GitHub Releases, verifies its
# checksum, backs up the current binary and performs an atomic replacement
# with automatic rollback if the new binary fails to start.
set -euo pipefail

REPO="${TTGTISO_REPO:-uiper123/nxDesk2.0}"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"
BIN_PATH="/usr/bin/ttgtiso-desk-agent"
BACKUP_PATH="${BIN_PATH}.bak"
SERVICE_NAME="ttgtiso-desk-agent"
ASSET_NAME="ttgtiso-desk-agent-linux-x86_64"
CHECKSUMS_NAME="SHA256SUMS"
WORK_DIR="$(mktemp -d /tmp/ttgtiso-update.XXXXXX)"
FORCE=false
CHECK_ONLY=false

cleanup() { rm -rf "$WORK_DIR"; }
trap cleanup EXIT

usage() {
  cat <<EOF
Usage: update-agent.sh [--check] [--force]

  --check   Only check whether a newer version is available (exit 0 = up to date, exit 10 = update available).
  --force   Reinstall even if the installed version is already up to date.

Environment:
  TTGTISO_REPO   Override GitHub repository (default: ${REPO})
EOF
}

for arg in "$@"; do
  case "$arg" in
    --check) CHECK_ONLY=true ;;
    --force) FORCE=true ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $arg" >&2; usage; exit 1 ;;
  esac
done

if [ "$CHECK_ONLY" = false ] && [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (sudo)." >&2
  exit 1
fi

command -v curl >/dev/null 2>&1 || { echo "Error: curl is required." >&2; exit 1; }

echo "Fetching latest release metadata for ${REPO}..."
RELEASE_JSON="$WORK_DIR/release.json"
curl -fsSL -H "Accept: application/vnd.github+json" "$API_URL" -o "$RELEASE_JSON"

LATEST_TAG="$(grep -m1 '"tag_name"' "$RELEASE_JSON" | sed -E 's/.*"tag_name"[^"]*"([^"]+)".*/\1/')"
LATEST_VERSION="${LATEST_TAG#v}"

if [ -z "$LATEST_VERSION" ]; then
  echo "Error: Could not determine the latest release version." >&2
  exit 1
fi

CURRENT_VERSION="0.0.0"
if [ -x "$BIN_PATH" ]; then
  CURRENT_VERSION="$("$BIN_PATH" --version 2>/dev/null | awk '{print $2}' || echo "0.0.0")"
fi

echo "Installed version: ${CURRENT_VERSION}"
echo "Latest version:    ${LATEST_VERSION}"

version_gt() {
  [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -n1)" = "$1" ]
}

if [ "$CHECK_ONLY" = true ]; then
  if version_gt "$LATEST_VERSION" "$CURRENT_VERSION"; then
    echo "Update available: ${CURRENT_VERSION} -> ${LATEST_VERSION}"
    exit 10
  fi
  echo "Agent is up to date."
  exit 0
fi

if ! version_gt "$LATEST_VERSION" "$CURRENT_VERSION" && [ "$FORCE" = false ]; then
  echo "Agent is already up to date. Use --force to reinstall."
  exit 0
fi

DOWNLOAD_BASE="https://github.com/${REPO}/releases/download/${LATEST_TAG}"

echo "Downloading ${ASSET_NAME}..."
curl -fsSL "${DOWNLOAD_BASE}/${ASSET_NAME}" -o "$WORK_DIR/$ASSET_NAME"

echo "Downloading ${CHECKSUMS_NAME}..."
if curl -fsSL "${DOWNLOAD_BASE}/${CHECKSUMS_NAME}" -o "$WORK_DIR/$CHECKSUMS_NAME"; then
  echo "Verifying checksum..."
  (cd "$WORK_DIR" && grep " ${ASSET_NAME}\$" "$CHECKSUMS_NAME" | sha256sum -c -)
else
  echo "Warning: ${CHECKSUMS_NAME} not found in release; skipping checksum verification." >&2
fi

chmod 755 "$WORK_DIR/$ASSET_NAME"

echo "Stopping ${SERVICE_NAME}..."
systemctl stop "$SERVICE_NAME" 2>/dev/null || true

if [ -f "$BIN_PATH" ]; then
  echo "Backing up current binary to ${BACKUP_PATH}..."
  cp -f "$BIN_PATH" "$BACKUP_PATH"
fi

echo "Installing new binary..."
install -m 755 "$WORK_DIR/$ASSET_NAME" "$BIN_PATH"

echo "Starting ${SERVICE_NAME}..."
systemctl daemon-reload
if ! systemctl start "$SERVICE_NAME"; then
  echo "Error: Service failed to start. Rolling back..." >&2
  if [ -f "$BACKUP_PATH" ]; then
    cp -f "$BACKUP_PATH" "$BIN_PATH"
    systemctl start "$SERVICE_NAME" || true
  fi
  exit 1
fi

sleep 3
if ! systemctl is-active --quiet "$SERVICE_NAME"; then
  echo "Error: Service is not active after update. Rolling back..." >&2
  systemctl stop "$SERVICE_NAME" 2>/dev/null || true
  if [ -f "$BACKUP_PATH" ]; then
    cp -f "$BACKUP_PATH" "$BIN_PATH"
    systemctl start "$SERVICE_NAME" || true
  fi
  exit 1
fi

echo "============================================="
echo "Update completed: ${CURRENT_VERSION} -> ${LATEST_VERSION}"
echo "Previous binary kept at: ${BACKUP_PATH}"
echo "============================================="
