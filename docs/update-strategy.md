# Update and Rollback Strategy

This document outlines the lifecycle of version updates, integrity checks, and crash rollback procedures.

---

## 1. Versioning & Package Delivery

We enforce semantic versioning (`MAJOR.MINOR.PATCH`). Updates are delivered in two formats:
1. **Direct Binary Replacement**: For quick patches and light configuration changes.
2. **System Debian Package (`.deb`)**: Recommended for production deployments.

---

## 2. Integrity Signatures & Validation

To prevent tampering and supply chain injections in secure environments:
* Every release bundle is hashed and signed with a private release GPG key.
* The public GPG key must be imported into the local target's keyring before installation.

### Manual verification:
```bash
# Verify checksum integrity
sha256sum -c SHA256SUMS

# Verify GPG signature
gpg --verify ttgtiso-desk-agent.tar.gz.asc ttgtiso-desk-agent.tar.gz
```

---

## 3. Rollback Action Plan

If an update fails, crashes on boot, or introduces regressions, a safe rollback is executed:

1. **Stop the active agent service**:
   ```bash
   sudo systemctl stop ttgtiso-desk-agent
   ```
2. **Restore binary from backup store**:
   The installer automatically keeps a backup of the previous binary at `/usr/bin/ttgtiso-desk-agent.bak`.
   ```bash
   sudo mv /usr/bin/ttgtiso-desk-agent.bak /usr/bin/ttgtiso-desk-agent
   ```
3. **Restore configuration file**:
   If changes to configuration caused instability, restore from `/etc/ttgtiso-desk/agent.toml.bak`.
   ```bash
   sudo cp /etc/ttgtiso-desk/agent.toml.bak /etc/ttgtiso-desk/agent.toml
   ```
4. **Restart the stable agent**:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl start ttgtiso-desk-agent
   ```
5. **Verify operations**:
   Confirm that the active version has reverted to the previous working build:
   ```bash
   journalctl -u ttgtiso-desk-agent -n 50 --no-pager
   ```

---

## 4. Automated Update System (GitHub Releases)

The project ships with a fully automated update pipeline built on GitHub Releases.

### How a release is published

1. Bump the version everywhere and create a tag:
   ```bash
   ./scripts/bump-version.sh 0.2.0
   git push origin main --tags
   ```
2. Pushing the `v*` tag triggers `.github/workflows/release.yml`, which:
   * builds the **server agent** for Linux x86_64 and uploads `ttgtiso-desk-agent-linux-x86_64`, `install-agent.sh`, `update-agent.sh` and `SHA256SUMS` to the release;
   * builds the **desktop client** with Tauri for Linux (`.deb`, `.rpm`, `.AppImage`) and Windows (`.msi`, `.exe`), signs the updater artifacts and publishes `latest.json` used by the in-app updater.

### Required GitHub secrets

| Secret | Purpose |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | Content of the updater private key (`updater-key`). |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the key (empty string if none). |

The matching public key is embedded in `apps/desktop-client/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). If you regenerate the keypair (`npx tauri signer generate -w updater-key`), update both the secret and the pubkey.

### Desktop client (in-app updates)

The client uses `tauri-plugin-updater`. The endpoint is configured to:

```
https://github.com/uiper123/nxDesk2.0/releases/latest/download/latest.json
```

In **Settings → Application Updates** the user can see the current version and press **Check for Updates**. The update is downloaded, its signature is verified against the embedded public key, installed, and the app relaunches automatically.

### Server agent (self-update)

The agent binary reports its version via `ttgtiso-desk-agent --version`. The update helper is installed as `/usr/bin/ttgtiso-desk-update`:

```bash
# check only (exit 10 = update available)
ttgtiso-desk-update --check

# download, verify SHA-256, atomically replace, restart, auto-rollback on failure
sudo ttgtiso-desk-update
```

Fresh installs can pull the latest release directly:

```bash
sudo ./scripts/install-agent.sh --from-github --unattended
```

For periodic automatic updates, add a cron entry or systemd timer:

```bash
echo '0 4 * * * root /usr/bin/ttgtiso-desk-update >> /var/log/ttgtiso-desk/update.log 2>&1' > /etc/cron.d/ttgtiso-desk-update
```

Rollback remains as described in section 3 — the updater keeps the previous binary at `/usr/bin/ttgtiso-desk-agent.bak` and restores it automatically if the new version fails to start.
