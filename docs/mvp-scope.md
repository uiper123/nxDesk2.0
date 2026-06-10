# MVP Scope — TTGTiSO-Desk

This document defines the scope of the Minimum Viable Product (MVP) for the TTGTiSO-Desk remote graphical access system.

## 1. Objectives

The primary goal of the MVP is to provide a secure, low-latency, multi-user remote desktop access system for closed local networks running **Astra Linux Special Edition 1.8 "Воронеж"** as the server platform.

## 2. In-Scope Features (MVP 1.0)

The following features must be implemented in the initial MVP release:

### 2.1. Server Agent
- **Platform Support:** Astra Linux SE 1.8 "Воронеж" (X11, Fly desktop).
- **Service Type:** Systemd daemon starting on boot.
- **Configuration:** Local configuration stored in `/etc/ttgtiso-desk/agent.toml`.
- **Logging:** Structured logging to journald and `/var/log/ttgtiso-desk/agent.log`.

### 2.2. Session Management
- **Multi-user Isolation:** Multiple users can log into separate, concurrent X11 sessions on the same server.
- **Session Lifecycle:** Programmatic session creation, querying, and destruction.
- **Isolation:** Isolated runtime directories and DBus session environments per user session.
- **Unattended Access:** Connections are established without requiring physical user confirmation on the server side.

### 2.3. Video Pipeline
- **Quality & Performance:** Target 1080p resolution at 30 FPS.
- **Encoding:** H.264 encoding with adaptive bitrate.
- **Hardware Acceleration:** VAAPI encoding with automatic software fallback (libx264/openh264).
- **Capture:** X11 capturing via SHM/XDamage.

### 2.4. Input & Control
- **Input Types:** Mouse movement, mouse clicks, mouse scrolling, and keyboard keystrokes.
- **Layouts:** Seamless Russian/English layout switching support.
- **Security Filters:** Intercepting/filtering dangerous system hotkey combinations (e.g., Ctrl+Alt+F1-F6, Ctrl+Alt+Del) depending on policy.

### 2.5. Transport & Protocol
- **Primary Transport:** SSH tunnel protocol (using SSH keys with password fallback).
- **Custom Protocol:** Binary multiplexed framing protocol (TTGTiSO Protocol) encapsulated within the secure SSH channel.
- **Logical Channels:** Separate virtual channels for Control, Video, Input, Clipboard, File, Audit, and Heartbeats.

### 2.6. Clipboard & File Transfer
- **Clipboard:** Bi-directional clipboard synchronization (text only).
- **File Transfer:** File upload and download capabilities over a secure file channel (SFTP or direct custom protocol channel).
- **Limits:** Strict configurable file size limits and transfer speed limits.

### 2.7. Security, Auditing & Roles
- **RBAC:** Four basic roles: `user`, `support_operator`, `admin`, `auditor`.
- **Security Logs:** Secure, tamper-resistant audit logs tracking session start/stop, file transfer events, and clipboard changes.
- **Hardening:** Disable insecure defaults; configuration permissions restricted to root.

### 2.8. Desktop Client
- **Platform Support:** Windows, Linux, and macOS.
- **Technology:** Tauri v2 + React + TypeScript.
- **Core Screens:** Login / Connection configuration, Host Management, Active Session view, File Transfer Manager, Settings, and Logs.

---

## 3. Out of Scope (For Future Releases)

The following items are explicitly **excluded** from the MVP:

- **Wayland support** (X11 only).
- **Audio redirection** (visuals and input only).
- **Multi-monitor support** (single monitor only).
- **Web-based client / WebRTC gateway** (Tauri desktop app only).
- **Automatic updates via public services** (offline deployment only).
- **LDAP / Active Directory / ALD Pro integration** (local system authentication and basic config mapping only).
- **Session recording** (video recordings of sessions).
- **Advanced Admin Web UI** (Admin CLI and basic local settings only).

---

## 4. MVP Acceptance Criteria (Definition of Done)

1. **Successful Installation:** The agent can be installed offline on Astra Linux SE 1.8 via `.deb` package or install script and runs as a systemd service.
2. **Concurrent Connections:** At least 3 independent users can connect simultaneously to the server and receive isolated X11 Fly desktop sessions.
3. **Target Video Stream:** Stream video at 1080p, maintaining at least 25-30 FPS under normal office workloads, with latency below 100ms on a local gigabit network.
4. **Input Control:** Mouse/keyboard inputs are successfully injected and mapped.
5. **Security Baselines:** SSH key authentication works; audit logs successfully register all connection and file transfer activities.
6. **Cross-Platform Client:** Client compiles and runs on Windows and Linux (Astra Linux client).
