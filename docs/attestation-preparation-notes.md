# Astra Linux K2 Attestation Preparation Notes

This document provides guidelines to prepare the TTGTiSO-Desk Remote Desktop system for government/corporate information security certification (K2 class attestation).

---

## 1. Compliance Requirements Checklist

To pass the K2 certification, the deployment must comply with the following policies:

### 1.1. Identification and Authentication (IA)
- [ ] **Restricted PAM Access**: Configure PAM `/etc/pam.d/sshd` to block connections from locked or unprivileged accounts.
- [ ] **Multi-Factor Authentication (MFA)**: Integrate PAM-MFA (such as Goolge Authenticator or local token providers) for SSH.

### 1.2. Access Control (AC)
- [ ] **RBAC Verification**: Validate client privileges using `crates/security/src/lib.rs` roles before establishing active display slots.
- [ ] **Isolated Display Allocator**: Confirm that `/tmp/.X11-unix/` directories cannot be read or modified by other unprivileged system users.

### 1.3. Security Audit & Monitoring (AU)
- [ ] **Audit Trail Integrity**: Verify that `AuditLog` records are written immediately to the local system log daemon (syslog/journald) and cannot be tampered with by the agent user.
- [ ] **External Log Ship**: In a production environment, configure `rsyslog` to automatically forward `/var/log/ttgtiso-desk/` logs to an external SIEM server.

---

## 2. Mandatory Verification Tasks

Prior to attestation audits:
1. Perform a full vulnerability scan on dependency crates:
   ```bash
   cargo audit
   ```
2. Validate that the custom input injector prevents any bypass of the security perimeter (e.g. blocking keystroke sequences like `Ctrl+Alt+F1` virtual console switching).
3. Ensure all binaries are signed with the corporate deployment key to guarantee integrity.
