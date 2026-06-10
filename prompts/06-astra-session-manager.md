# Prompt 06 — Astra Linux X11 Session Manager

Implement isolated graphical sessions for users.

## Goal

Each connected user must receive a separate isolated X11 graphical session.

## Requirements

- Astra Linux SE 1.8 priority;
- Fly desktop priority;
- X11 only for MVP;
- KDE/GNOME optional through abstraction;
- no Wayland requirement for MVP;
- PAM/system user integration where appropriate;
- session create/start/stop/status APIs;
- resource cleanup;
- audit events.

## Required Crates

- `crates/session-manager`
- `crates/os-pal`

## Required Interfaces

Create interfaces for:

- `SessionManager`
- `SessionBackend`
- `UserSession`
- `DisplayAllocator`
- `DesktopLauncher`
- `SessionLifecycle`
- `SessionAuditSink`

## Required Backends

- mock backend for tests;
- Astra X11 backend;
- placeholder generic Linux X11 backend.

## Tests

- create session;
- stop session;
- duplicate user handling;
- cleanup after crash;
- display allocation.
