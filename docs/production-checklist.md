# Production Readiness Checklist

Before moving the TTGTiSO-Desk Remote Desktop system to active production, verify all criteria below are satisfied.

---

## 1. Security Controls Verification

- [ ] **Config Permissions Check**: Confirm `/etc/ttgtiso-desk/agent.toml` is `chmod 600` and owned by root.
- [ ] **State Path Permissions Check**: Verify `/var/lib/ttgtiso-desk/` is `chmod 700`.
- [ ] **Audit Logging Enabled**: Confirm `enable_audit_logs = true` is set in configuration.
- [ ] **No Weak Cryptography**: Ensure SSH host keys on the Astra Server use modern cryptosystems (e.g. ED25519 or RSA 4096).
- [ ] **SSH Passwordless Authentication**: Switch `allow_password_auth` to `false` in `agent.toml` to enforce SSH key pair authentication.

---

## 2. Infrastructure & Systemd Integration

- [ ] **Hardened Service Sandbox**: Verify systemd sandbox directives (`ProtectSystem`, `PrivateTmp`, `NoNewPrivileges`) are active in the unit definition.
- [ ] **Auto-Restart & Watchdog**: Ensure systemd restart configuration (`Restart=always`, `RestartSec=5`) is set to recover from failures automatically.
- [ ] **Log Rotation Setup**: Confirm that `/etc/logrotate.d/ttgtiso-desk` is created and active.

---

## 3. Network Constraints

- [ ] **Single-Port SSH Rule**: Verify that only port 22 is exposed externally, and no other ports are open.
- [ ] **Relay Authentication**: If utilizing the relay server, confirm that unique tokens (`agent_token`, `client_token`) are set and rotated.
