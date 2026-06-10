# Security Architecture & Key Management — TTGTiSO-Desk

This document specifies the security implementation details, authentication procedures, and credential management for TTGTiSO-Desk.

## 1. Authentication Mechanisms

TTGTiSO-Desk uses SSH for the transport layer and delegates primary authentication to the SSH server daemon or the server agent's SSH library.

### 1.1. SSH Key-Based Authentication (Primary)
- **Mechanism:** Public/Private key pair exchange.
- **Client Side:** Private keys are loaded from the user's home directory (`~/.ssh/id_ed25519` or `~/.ssh/id_rsa`) or from the client machine's native SSH agent.
- **Server Side:** Authorized public keys are stored in the user's `~/.ssh/authorized_keys` file.

### 1.2. Password Authentication (Fallback)
- **Mechanism:** Secure PAM (Pluggable Authentication Modules) authentication.
- **Rules:** Cleartext passwords are encrypted in transit via the SSH channel. They must **never** be written to logs or configurations.

---

## 2. Secure Credential Storage (Client)

The Tauri desktop client must not write plain password or private key passphrases to configuration files on the disk.

- **Platform-Native Keychain:** TTGTiSO-Desk integrates the `keyring` crate to store credentials securely:
  - **Linux:** Secret Service API (via D-Bus) or Gnome Keyring.
  - **Windows:** Credential Manager.
  - **macOS:** Keychain Services.
- **Entry Key Format:** `ttgtiso-desk:{username}@{host}`

---

## 3. Host Key Verification

To prevent Man-in-the-Middle (MitM) and Spoofing attacks:
- **First Connection:** The client prompts the user with the server's public key fingerprint (TOFU - Trust On First Use).
- **Subsequent Connections:** The client compares the server's host key against the stored signature in `~/.ttgtiso-desk/known_hosts`.
- **Mismatches:** If the host key has changed, the client aborts the connection automatically, displaying a critical security warning.

---

## 4. Secret Net Studio Integration Points

In highly secure Astra Linux environments, Secret Net Studio (SNS) enforces information classification and integrity control. TTGTiSO-Desk supports SNS via these hooks:
1. **Dynamic Session Classification:** The `session-manager` reads the current user's security level (confidentiality tier) on startup and passes it to the `audit-log` crate.
2. **File Transfer Constraints:** Before writing chunks, `file-transfer` triggers a command-line utility query or reads classification metadata from the target directory to verify if the file classification matches the user's session tier.
3. **Session Interception:** A system-wide PAM module managed by SNS can dynamically deny the SSH session setup if security policies are violated.
