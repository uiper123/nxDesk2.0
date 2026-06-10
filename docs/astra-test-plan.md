# Astra Linux VM Testing Plan

This document provides a manual verification test plan for QA operators deploying and validating TTGTiSO-Desk within an Astra Linux VM or bare-metal environment.

---

## 1. Prerequisites and Environment Setup

### Target VM OS:
* Astra Linux Special Edition 1.7 or 1.8.
* Role: "Воронеж" (Workstation) or "Смоленск" (Hardened Server) with standard Fly window manager installed.

### Verification Tools:
* `x11-utils` (specifically `xdpyinfo` and `xwininfo`) to inspect display allocations.
* `htop` / `ps` to monitor processes.
* `/var/log/ttgtiso-desk/` logs folder.

---

## 2. Test Scenarios

### Test Scenario 1: Unattended Installer Execution
1. Copy the offline deployment tarball to the target Astra VM.
2. Run the script:
   ```bash
   sudo ./scripts/install-agent.sh --unattended
   ```
3. **Verify**:
   - Check that `ttgtiso-desk-agent` binary exists in `/usr/bin/`.
   - Check that folders `/etc/ttgtiso-desk/`, `/var/lib/ttgtiso-desk/`, and `/var/log/ttgtiso-desk/` are created with correct Unix permissions (`0700` and `0600`).
   - Check that the systemd service is active:
     ```bash
     systemctl is-active ttgtiso-desk-agent
     ```

### Test Scenario 2: Multi-User Virtual Display Isolation
1. Authenticate two separate user connections (e.g. `user1` on display `:10`, `user2` on display `:11`).
2. **Verify**:
   - Each session spawns its own `Xvfb` and `fly-wm` processes.
   - Run `ps aux | grep Xvfb` and verify that command line displays match `:10` and `:11`.
   - Verify that `user1` cannot see or interact with the graphical session of `user2` (strict process and display isolation).

### Test Scenario 3: Clipboard Sync & Hardening
1. Copy text on the client workstation. Confirm it is synchronized to the remote Astra graphical environment.
2. Try copying a block of text larger than 1 MB.
3. **Verify**:
   - The large sync is blocked by the clipboard policy limit.
   - Check `/var/log/ttgtiso-desk/` (or journalctl) for a `CLIPBOARD_VIOLATION` audit event.

### Test Scenario 4: Input Control Policy Enforcement
1. In the active remote session, press hotkeys designed to break out of the session (e.g. `Ctrl+Alt+F1` through `Ctrl+Alt+F12`, `Ctrl+Alt+Del`).
2. **Verify**:
   - Key inputs are safely captured, validated, and filtered out by the `SecureInputPolicy`.
   - No virtual terminal switching occurs on the remote Astra host.

### Test Scenario 5: File Transfer and Executable Denial
1. Use the Connection panel to upload a safe file (e.g. `report.txt`).
2. Use the Connection panel to upload an executable script (e.g. `exploit.sh`).
3. **Verify**:
   - `report.txt` uploads successfully, and is written to the state directory.
   - `exploit.sh` is rejected immediately with a policy error.
   - Review audit records to ensure `FILE_TRANSFER_VIOLATION` is written.
