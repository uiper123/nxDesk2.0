# Known Limitations (MVP 1.0)

This document lists the limitations of the current MVP release.

---

## 1. Out-of-Scope Features

The following features were explicitly excluded from the MVP 1.0 scope:
1. **Remote Audio Redirection**: Real-time remote audio streaming is currently disabled.
2. **Multi-Monitor Support**: Video pipeline captures only the primary virtual display.
3. **Web Client Access**: A dedicated desktop client (Tauri wrapper) is required; browser-based access is not supported.
4. **Session Video Recording**: Recording and archiving session activities on the server side is not supported.
5. **LDAP/Active Directory Direct Authentication Integration**: Authentication relies entirely on standard PAM/SSH authentication mapping.

---

## 2. Hard Limits & Network Constraints

* **Maximum Concurrent Sessions**: Limited to 5 concurrent sessions per agent by default config to conserve CPU/memory resources on server hosts.
* **X11 Display Allocation Range**: Restricted to display IDs `:10` through `:99`.
* **Video Encoding Fallback**: Hardware encoding uses VAAPI H.264. Software fallback uses OpenH264, which is CPU-intensive on older multi-core servers.
* **Text-Only Clipboard Sync**: Rich elements such as images or raw HTML clipboard items are parsed but may undergo conversions depending on the active client system capabilities.
