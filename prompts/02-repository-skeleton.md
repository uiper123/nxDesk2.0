# Prompt 02 — Repository Skeleton

Create the initial TTGTiSO-Desk monorepo skeleton.

## Rules

CRITICAL:
- Do not implement business logic yet.
- Create only structure, configs, interfaces and minimal compile-ready stubs.
- Each file must be below 250 lines.
- Before every generated code block, print the full file path.
- One file = one responsibility.

## Required Stack

- Rust workspace.
- Tauri v2 desktop client.
- React + TypeScript frontend.
- Shared TypeScript types.
- Rust crates for core modules.
- Basic CI.
- Basic formatting/linting configs.

## Required Directories

Create:

- `apps/desktop-client`
- `apps/server-agent`
- `apps/relay-server`
- `apps/admin-cli`
- `crates/protocol`
- `crates/transport`
- `crates/security`
- `crates/session-manager`
- `crates/video-pipeline`
- `crates/input-injector`
- `crates/clipboard`
- `crates/file-transfer`
- `crates/audit`
- `crates/config`
- `crates/os-pal`
- `crates/shared-types`
- `packages/ui`
- `packages/shared-types`
- `docs`
- `tests`
- `packaging`
- `scripts`

## Definition of Done

- `cargo check` passes.
- frontend dependency structure is valid.
- all crates compile as stubs.
- basic README exists.
- CI skeleton exists.
