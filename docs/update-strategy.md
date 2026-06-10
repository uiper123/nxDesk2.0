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
