# Prompt 04 — Security Baseline

Implement the security foundation for TTGTiSO-Desk.

## Security Model

Required:
- SSH key authentication;
- password authentication as fallback;
- host key verification;
- secure credential storage on client;
- RBAC;
- audit log;
- encrypted transport;
- secure configuration defaults;
- Secret Net Studio integration extension points.

## Roles

Implement roles:

- user
- admin
- support_operator
- auditor

## Required Crates

- `crates/security`
- `crates/audit`
- `crates/config`

## Required Documents

- `docs/security.md`
- `docs/threat-model.md`
- `docs/audit-policy.md`

## Rules

- Never log secrets.
- Never store plaintext passwords.
- Use keychain/credential storage where available.
- All sensitive channels must be encrypted through SSH.
- Audit metadata, not clipboard/file contents by default.

## Tests

Add:
- RBAC tests;
- auth config tests;
- secret redaction tests;
- audit event tests.
