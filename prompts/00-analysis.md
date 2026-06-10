# Prompt 00 — Initial Analysis for TTGTiSO-Desk

You are a Principal Systems Architect and Senior Full-Stack Developer.

Project: TTGTiSO-Desk.

Before writing any production code, perform a deep technical analysis of the system.

## Product Goal

Build a secure, high-performance, multi-user remote desktop system for closed local networks.

The priority server platform is Astra Linux Special Edition 1.8 “Воронеж” with X11 and Fly desktop.

The client must be cross-platform: Windows, Linux, macOS.

The main use case:
- several users work on one Astra Linux server;
- each user gets a separate isolated graphical session;
- connection is possible without confirmation from the remote user;
- the connection must be secure;
- RDP and VNC must not be used as the main protocol.

## Required Output

Create these documents:

- `docs/mvp-scope.md`
- `docs/architecture.md`
- `docs/threat-model.md`
- `docs/protocol.md`
- `docs/risks.md`
- `docs/adr/0001-core-technology-stack.md`
- `docs/adr/0002-transport-protocol.md`
- `docs/adr/0003-astra-linux-session-strategy.md`

## Architecture Defaults

Use these defaults unless you find a strong reason not to:

- Core language: Rust.
- Desktop client: Tauri v2 + React + TypeScript.
- Main transport: SSH + custom multiplexed protocol.
- File transfer: SFTP or secure file channel over SSH.
- Video: X11 capture + GStreamer + H.264.
- Encoder: VAAPI when available, software fallback required.
- Server agent: systemd service.
- Target FPS: 1080p 30 FPS adaptive.
- Priority: low latency and security.
- Offline mode: mandatory.
- Internet access: must not be required.
- Relay/jump-host support: required for restricted networks.
- Web client: optional future feature, not MVP.

## Important Rules

Do not write production code yet.

First analyze:
1. system architecture;
2. security model;
3. protocol design;
4. X11/Fly/Astra Linux risks;
5. MVP scope;
6. test strategy;
7. deployment strategy.

Use Mermaid diagrams where useful.

End with a clear implementation roadmap.
