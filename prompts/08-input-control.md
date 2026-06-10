# Prompt 08 — Remote Input Control

Implement keyboard and mouse control for remote sessions.

## Goal

Send client input events to a remote X11 session.

## Required Crate

- `crates/input-injector`

## Required Events

- mouse move;
- mouse down;
- mouse up;
- scroll;
- key down;
- key up;
- text input;
- hotkey;
- layout-aware key mapping.

## Security Requirements

- filter dangerous host-level key combinations;
- session-scoped input only;
- audit input metadata if required;
- never leak input across sessions.

## Interfaces

Create:

- `InputInjector`
- `InputEvent`
- `KeyboardMapper`
- `MouseMapper`
- `InputPolicy`
- `InputAuditSink`

## Backends

- mock backend;
- X11 backend.

## Tests

- key mapping tests;
- mouse event tests;
- forbidden hotkey tests;
- session isolation tests.
