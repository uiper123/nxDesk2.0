# Prompt 01 — Architecture Design

You are designing the production architecture of TTGTiSO-Desk.

## Goal

Create a clean modular architecture for a secure remote desktop system.

## Required Architecture

Use a monorepo:

/ttgtiso-desk
├── apps/
│   ├── desktop-client/
│   ├── server-agent/
│   ├── relay-server/
│   └── admin-cli/
├── crates/
│   ├── protocol/
│   ├── transport/
│   ├── security/
│   ├── session-manager/
│   ├── video-pipeline/
│   ├── input-injector/
│   ├── clipboard/
│   ├── file-transfer/
│   ├── audit/
│   ├── config/
│   ├── os-pal/
│   └── shared-types/
├── packages/
│   ├── ui/
│   └── shared-types/
├── docs/
├── packaging/
├── scripts/
└── tests/

## Architectural Style

Use:
- clean architecture;
- ports/adapters;
- explicit interfaces first;
- strict modularity;
- single responsibility per file.

## Output

Create/update:

- `docs/architecture.md`
- `docs/module-boundaries.md`
- `docs/component-diagram.md`
- `docs/data-flow.md`

Include Mermaid diagrams for:
- system overview;
- client-agent-relay interaction;
- session lifecycle;
- protocol channel flow.

## Rules

Do not create large files.
No file may exceed 250 lines.
Do not create God Objects.
Each module must have clear interfaces before implementation.
