# Desktop Client UI Architecture

This document details the cross-platform client shell interface implemented using Tauri v2, React, and TypeScript.

## Technology Stack

- **Framework**: Tauri v2 (providing lightweight cross-platform native system integration).
- **Frontend library**: React 19.
- **Language**: TypeScript (ensuring type safety matching Rust structures).
- **Styles**: Scoped CSS modules (guaranteeing component isolation).

## Screen Map

1. **Login Screen**:
   - Operator credentials input (password, private keys).
   - Validation against destination hosts.

2. **Host List Registry**:
   - Multi-host status overview (online, offline, busy).
   - Lists active sessions count.

3. **Connection Progress Card**:
   - Real-time handshake log trace.
   - Status updates of virtual X11 allocations.

4. **Active Session Display**:
   - Streaming desktop canvas overlay.
   - Latency, FPS, and compression bitrate telemetry overlay.
   - Scale, Fullscreen, and Disconnect options.
   - Hotkey security filter validation indicators.

5. **Settings Panel**:
   - Quality presets, hardware acceleration toggles, and audio streaming controls.

6. **Admin Panel**:
   - Active user session registration inspection and force-kill controls.

7. **Audit Logs Panel**:
   - Local connection diagnostics and input policy logs.

## Component Directory Structure

All components follow the isolated UI guidelines:
```text
ComponentName/
├── ComponentName.tsx          # React logic and DOM
├── ComponentName.module.css   # Component-specific styles
├── ComponentName.test.tsx     # Unit tests
└── index.ts                   # Public component exports
```
