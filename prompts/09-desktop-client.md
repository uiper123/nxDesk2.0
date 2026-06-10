# Prompt 09 — Desktop Client UI

Implement the cross-platform desktop client.

## Stack

- Tauri v2
- React
- TypeScript
- minimal UI
- light/dark theme

## Screens

Implement screens:

- Login
- Host List
- Connection Card
- Active Session
- File Transfer
- Settings
- Logs
- Admin Panel

## UX Requirements

- simple corporate minimalism;
- fast connection flow;
- fullscreen mode;
- scaling controls;
- reconnect state;
- session toolbar;
- file transfer panel;
- clipboard status;
- connection quality indicator.

## Architecture

Use isolated UI components.

Each component must have its own directory:

ComponentName/
├── ComponentName.tsx
├── ComponentName.module.css
├── ComponentName.test.tsx
└── index.ts

## Tauri Integration

Use typed commands.
Generate TypeScript bindings from Rust types where possible.

## Tests

- component tests;
- state tests;
- command mock tests.
