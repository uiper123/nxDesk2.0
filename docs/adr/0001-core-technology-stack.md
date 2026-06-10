# ADR 0001: Core Technology Stack

## Status
Accepted

## Context
TTGTiSO-Desk requires a secure, high-performance, and resource-efficient remote desktop solution. The server must run on Astra Linux SE 1.8 (X11 environment), supporting multiple concurrent user sessions. The client must be cross-platform (Windows, macOS, Linux). The system operates in closed local networks, meaning it must be distributed as self-contained binaries with no dependencies on external packet registries during execution.

## Considered Alternatives
1. **C++ / Qt:** Highly performant, but susceptible to memory safety issues. Qt licensing can be complex, and setup is heavier.
2. **Electron / Node.js:** Easy to develop, but memory-intensive and results in massive installer bundles (150MB+), which is highly undesirable in secure restricted environments.
3. **Rust + Tauri v2 + React + TypeScript:** Tauri compiles to native binaries using the operating system's native webview. The backend is written in Rust, which guarantees memory safety and speed, while the frontend is built using standard React + TypeScript.

## Decision
We choose **Rust + Tauri v2 + React + TypeScript** as the core technology stack:
- **Server Agent & Relay:** Pure Rust CLI applications compiled staticly where possible.
- **Client Desktop App:** Tauri v2 with a Rust backend and a React/TypeScript frontend.
- **Type Safety:** Use `specta` / `tauri-specta` to export Rust types to TypeScript definitions, ensuring compile-time safety across boundaries.

## Consequences
- **Pros:**
  - Extremely lightweight client installer (<15MB) and low memory usage.
  - Native speed and memory safety on the server agent (written in Rust).
  - Rapid UI development using React and CSS modules.
  - No runtime dependency on Node.js or heavy web browser runtimes.
- **Cons:**
  - Requires developers to be proficient in both Rust and TypeScript.
  - System native webview is required on the client machine (WebView2 on Windows, WebKitGTK on Linux, WebKit on macOS).
