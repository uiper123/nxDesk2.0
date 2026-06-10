# Prompt 05 — Server Agent

Implement the TTGTiSO server agent.

## Target Platform

Primary:
- Astra Linux Special Edition 1.8 “Воронеж”
- X11
- Fly desktop

## Agent Requirements

- run as systemd service;
- auto-start after reboot;
- load config from `/etc/ttgtiso-desk/agent.toml`;
- store runtime data in `/var/lib/ttgtiso-desk/`;
- write logs to journald and `/var/log/ttgtiso-desk/`;
- expose local control through Unix socket;
- accept secure client connections;
- manage user sessions;
- support offline closed networks.

## Required App

- `apps/server-agent`

## Required Crates

- `crates/config`
- `crates/audit`
- `crates/session-manager`
- `crates/transport`
- `crates/security`

## Deliverables

- service entrypoint;
- config loader;
- lifecycle manager;
- graceful shutdown;
- health status;
- mock connection handler.

## Tests

- config parsing;
- startup validation;
- graceful shutdown;
- audit event writing.
