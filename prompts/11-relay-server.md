# Prompt 11 — Relay Server and Bastion Mode

Implement internal relay support for closed networks.

## Goal

Allow connections when direct client-to-agent access is impossible.

## Required Modes

- direct SSH;
- SSH through jump host;
- internal relay server;
- single-port mode.

## Required App

- `apps/relay-server`

## Required Features

- connection registration;
- agent presence;
- client routing;
- heartbeat;
- audit events;
- no unnecessary decryption at relay;
- offline deployment;
- configurable bind address and port.

## Security

- relay must authenticate clients and agents;
- relay must not log secrets;
- relay should minimize access to session contents.

## Tests

- client-agent relay connection;
- heartbeat timeout;
- unauthorized connection rejection;
- reconnect behavior.
