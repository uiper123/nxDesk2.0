# ADR 0003: Astra Linux Session Strategy

## Status
Accepted

## Context
A key requirement for TTGTiSO-Desk is supporting multiple concurrent, isolated graphical user sessions on a single Astra Linux SE 1.8 server. Connections must be established without requiring physical user confirmation at the local terminal, and each connected user must be fully isolated from others.

## Considered Alternatives
1. **Scraping Active Display (:0) via VNC/X11:** Does not support multiple isolated users (all users see the same screen, and it requires a physical user to be logged in).
2. **XRDP / X11rdp:** Standard RDP server for Linux. However, it is difficult to integrate custom low-latency video pipelines (like GStreamer H.264 with VAAPI), has poor support for audit telemetry, and makes input layout intercepting hard to coordinate.
3. **Dedicated Virtual X11 Server (Xvfb / Xorg dummy) + Fly Desktop:** The agent programmatically spawns a virtual X server on an unused display ID (e.g. `:10`) under the authenticated user's credentials, sets up a clean DBus session, launches the Fly window manager (`fly-wm`), and attaches the GStreamer capture pipe directly to that display.

## Decision
We choose **spawning a dedicated Virtual X11 Server (Xvfb or Xorg with dummy/xvfb drivers) + Fly Desktop** for each user session:
- The `session-manager` handles display ID allocation, ensuring no collisions.
- Spawns the virtual display server under the target Unix user account context using `sudo -u` or PAM hooks.
- Launches the Fly window manager environment.
- The `video-pipeline` captures frames from this specific `:X` display using X11 shared memory (MIT-SHM) or XDamage.

## Consequences
- **Pros:**
  - True multi-user isolation: each user receives their own independent desktop environment.
  - Unattended session startup: no physical terminal interaction needed.
  - Direct control over the capture pipeline (GStreamer captures native frame buffers).
  - Compatibility with standard Astra Linux Fly configurations.
- **Cons:**
  - Spawning a full window manager per user session consumes server RAM and GPU resources.
  - Virtual framebuffers lack 3D acceleration unless virtual GL tools (like VirtualGL) are configured.
