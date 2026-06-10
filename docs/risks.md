# Risk Assessment & Mitigations — TTGTiSO-Desk

This document outlines the key technical, security, and operational risks associated with developing the TTGTiSO-Desk remote access system, along with strategies to mitigate them.

## 1. Technical & Engineering Risks

### 1.1. X11 Session Creation & Management on Astra Linux 1.8
- **Risk:** Astra Linux Special Edition 1.8 uses a customized Fly Desktop environment. Spawning multi-user isolated sessions programmatically via a daemon running as `root` can cause:
  - Display ID conflicts (e.g., trying to use `:1` when it's already allocated).
  - DBus session environment contamination.
  - PAM authentication failures when executing outside the DM (Display Manager).
- **Mitigation:** Implement a stateful Display Allocator in `session-manager` to track active displays dynamically (starting from `:10` up to `:99`). Use proper PAM session setup routines (`pam_open_session`) and clean environment setups when starting the Fly window manager inside a virtual framebuffer (Xvfb / Xorg).

### 1.2. GStreamer H.264 Encoder Capabilities
- **Risk:** Hardware acceleration (VAAPI, NVENC) might not be available or stable on all server host machines in secure local networks. Falling back to software encoding (libx264/openh264) can consume significant CPU resources under multi-user load.
- **Mitigation:** Design an adaptive encoder selector. Test the GStreamer pipeline on startup with a mock buffer using VAAPI; if it fails, fallback immediately to openh264/x264 with CPU presets optimized for low latency (e.g., `preset=ultrafast`, `tune=zerolatency`).

### 1.3. Keyboard Layout & Input Mapping
- **Risk:** Translating keypresses from diverse client OS layouts (Windows, macOS, Linux) to the target X11 system in Astra Linux often causes layout mismatch errors (e.g., typing Russian letters results in English characters, or hotkeys don't trigger).
- **Mitigation:** Send standard X11 keysyms from the Tauri client instead of scan codes. Implement an input translator that understands both layouts and injects keys directly using the X11 Test extension (`XTest`), which bypasses client-side local keyboard layout transformations on the server.

---

## 2. Security & Compliance Risks

### 2.1. Secret Net Studio & Astra Security Policies
- **Risk:** Closed networks running Astra Linux SE 1.8 often deploy local security enforcement tools like Secret Net Studio, Parsec, or strict AppArmor profiles. The Server Agent could be flagged or blocked from running system commands, spawning X processes, or binding to SSH tunnels.
- **Mitigation:** Build the agent using standard systemd design patterns. Deliver a clear security registry outlining all system calls, directories accessed (`/etc/ttgtiso-desk/`, `/var/lib/ttgtiso-desk/`), and standard ports used. Keep the installation path within standard FHS locations.

### 2.2. Unauthorized Session Access
- **Risk:** Since sessions connect without target user confirmation (unattended access), a support operator or admin could theoretically view or hijack a session of another active user.
- **Mitigation:**
  - Enforce RBAC logic.
  - Implement mandatory audit logging: every single session request creates an unalterable log entry.
  - Optional setting in `agent.toml` to prompt target user confirmation if they are already logged in locally.
  - Cleanly destroy all files and session environments on disconnect unless "persist session" is explicitly allowed and configured.
