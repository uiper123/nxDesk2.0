# Audit Policy — TTGTiSO-Desk

This document defines the auditing standards, event classification, and tamper-resistance measures implemented in the auditing subsystem of TTGTiSO-Desk.

## 1. Core Auditing Principles

1. **Non-Repudiation:** Every remote connection, file transfer, and admin operation must leave an immutable record.
2. **Metadata-Only Logging:** To prevent data leaks, the clipboard and file transfer logs must only log metadata (size, file names, directions), **never** actual text or file content.
3. **Zero Secrets Logging:** Passwords, private keys, or API tokens must be strictly redacted from any trace output or log statement.
4. **Local and System Integration:** Logs are mirrored to local secure files (`/var/log/ttgtiso-desk/audit.log`) and standard Unix log facilities (`journald` / syslog).

---

## 2. Audit Event Matrix

| Event Code | Event Type | Severity | Parameters Logged | Description |
| :--- | :--- | :--- | :--- | :--- |
| **AUTH_01** | AUTH_SUCCESS | INFO | `username`, `remote_ip`, `auth_method` | Successful client SSH login. |
| **AUTH_02** | AUTH_FAILURE | WARNING | `username`, `remote_ip`, `reason` | Failed login attempt. |
| **SESS_01** | SESSION_START | INFO | `session_id`, `username`, `display` | Isolated X11 session creation. |
| **SESS_02** | SESSION_STOP | INFO | `session_id`, `username`, `reason` | Virtual X11 session termination. |
| **FILE_01** | FILE_TX_START | INFO | `transfer_id`, `file_name`, `size`, `is_upload` | File transfer requested. |
| **FILE_02** | FILE_TX_END | INFO | `transfer_id`, `success`, `bytes_written` | File transfer completed. |
| **CLIP_01** | CLIPBOARD_SYNC | INFO | `session_id`, `char_count`, `direction` | Clipboard text synced. |
| **CONF_01** | CONFIG_CHANGE | CRITICAL | `param_name`, `old_val`, `new_val`, `admin_user` | Configuration file reloaded/edited. |

---

## 3. Redaction Patterns

To enforce the "Never Log Secrets" rule, the audit system automatically applies the following regex filters before writing logs:
- **Password Redaction:** `password\s*=\s*"[^"]*"` is replaced by `password = "[REDACTED]"`.
- **Private Key Passphrase:** `passphrase\s*=\s*"[^"]*"` is replaced by `passphrase = "[REDACTED]"`.

---

## 4. Log Integrity & Tamper Protection

- **Access Controls:** The log folder `/var/log/ttgtiso-desk/` is owned by `root:ttgtiso-desk` with permission mode `0750`. Log files are created with mode `0600` (readable only by root).
- **Append-Only Enforcement:** Files are opened with the `O_APPEND` flag. In production environments, administrators are advised to configure log rotation to mirror logs immediately to a central syslog server.
