# Astra Linux Installation Guide

This document describes how to deploy, configure, and secure the **TTGTiSO-Desk Remote Desktop Server Agent** on Astra Linux Special Edition (version 1.7 / 1.8).

---

## 1. System Paths and Structure

The installation script places system components in the following standardized directories:

| Component | Path | Permissions | Purpose |
| :--- | :--- | :--- | :--- |
| **Binary** | `/usr/bin/ttgtiso-desk-agent` | `0755` (root:root) | Executable binary file |
| **Configuration** | `/etc/ttgtiso-desk/agent.toml` | `0600` (root:root) | Global configuration |
| **State Data** | `/var/lib/ttgtiso-desk/` | `0700` (root:root) | Session allocations, temp files |
| **Log Files** | `/var/log/ttgtiso-desk/` | `0700` (root:root) | Audit logs and debug traces |
| **Systemd Service** | `/etc/systemd/system/ttgtiso-desk-agent.service` | `0644` (root:root) | Systemd manager definition |

---

## 2. Interactive and Unattended Installation

### Interactive Mode
Run the installer script without parameters. If the binary is not automatically located in build targets, you will be prompted to supply a path:
```bash
sudo ./scripts/install-agent.sh
```

### Unattended Mode (CI/CD / Ansible)
To run automated deployments without prompting for interactive input, use the `--unattended` flag. The script expects the pre-built `server-agent` binary to reside in either the current directory or `./target/release/`:
```bash
sudo ./scripts/install-agent.sh --unattended
```

---

## 3. Configuration & Hardening Checklist

The file `/etc/ttgtiso-desk/agent.toml` governs the security perimeter:
* **`allow_password_auth`**: Set to `false` in highly secure environments to restrict connections to SSH key pairs exclusively.
* **`enable_audit_logs`**: Must remain `true` to ensure full coverage of authentication and data sync actions.

### Hardening Checklist

- [ ] **Restrict Config Access**: Confirm that `/etc/ttgtiso-desk/agent.toml` is set to `chmod 600` so unprivileged users cannot read session configuration.
- [ ] **Restrict Directory Access**: Confirm that state and log folders (`/var/lib/ttgtiso-desk` and `/var/log/ttgtiso-desk`) are set to `chmod 700`.
- [ ] **Systemd Hardening Sandbox**: The systemd service template limits process capabilities:
  * `ProtectSystem=full` mounts `/usr`, `/boot`, and `/etc` as read-only.
  * `NoNewPrivileges=true` blocks executing child binaries with escalated privileges.
  * `ProtectHome=read-only` shields user directories from alterations.

---

## 4. Troubleshooting & Verification

To verify that the agent is running correctly:
```bash
# Check service status
sudo systemctl status ttgtiso-desk-agent

# Monitor real-time logs
sudo journalctl -u ttgtiso-desk-agent -f
```
