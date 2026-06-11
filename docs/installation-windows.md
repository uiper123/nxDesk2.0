# Windows Server Agent — Installation Guide

This guide covers installing the **TTGTiSO-Desk server agent** on Windows so it
runs as an auto-start background service (survives reboots and user logoff),
analogous to the systemd service used on Astra Linux.

## What you get

- The agent installed as a Windows **service** named `TTGTiSODeskAgent`.
- **Automatic start at boot** under the `LocalSystem` account — no user needs to
  be logged in for the service process to run.
- **Crash recovery**: the Service Control Manager restarts the agent if it exits
  unexpectedly.
- Screen capture via the Win32 **GDI** API and input injection via **SendInput**
  (no GStreamer/X11 needed on Windows). Frames use the same PNG wire format as
  the Linux software path, so the existing desktop client works unchanged.

> Note on session scope: the agent runs as a service in the background and
> always works. Capturing the interactive desktop and injecting input requires
> an active, unlocked console session, which is the normal Windows constraint
> for any remote-desktop helper. The agent process itself stays running
> regardless; capture simply targets the current console desktop.

## Requirements

- Windows 10 / 11 or Windows Server 2019+ (x86_64).
- Administrator privileges to install the service.

## Quick install (from a GitHub release)

Open **PowerShell as Administrator** and run:

```powershell
# Downloads the latest agent .exe and registers the auto-start service
powershell -ExecutionPolicy Bypass -File install-agent.ps1
```

To pin a specific release tag:

```powershell
powershell -ExecutionPolicy Bypass -File install-agent.ps1 -Version v0.2.0
```

## Install from a locally built binary

Build the Windows binary (on Windows with the MSVC toolchain, or cross-compile):

```powershell
cargo build --release -p server-agent
```

Then install it:

```powershell
powershell -ExecutionPolicy Bypass -File install-agent.ps1 -BinaryPath .\target\release\server-agent.exe
```

## File locations

| Item    | Path                                                        |
| ------- | ----------------------------------------------------------- |
| Binary  | `C:\Program Files\TTGTiSO-Desk\ttgtiso-desk-agent.exe`      |
| Config  | `C:\ProgramData\TTGTiSO-Desk\agent.toml`                    |
| Logs    | `C:\ProgramData\TTGTiSO-Desk\logs\audit.log`                |
| Service | `TTGTiSODeskAgent` (auto-start, LocalSystem)                |

## Managing the service

```powershell
# Status
Get-Service TTGTiSODeskAgent

# Stop / start / restart
Stop-Service  TTGTiSODeskAgent
Start-Service TTGTiSODeskAgent
Restart-Service TTGTiSODeskAgent

# View recent events
Get-EventLog -LogName Application -Source TTGTiSODeskAgent -Newest 20
```

The agent also exposes the same controls directly:

```powershell
& "C:\Program Files\TTGTiSO-Desk\ttgtiso-desk-agent.exe" --install-service
& "C:\Program Files\TTGTiSO-Desk\ttgtiso-desk-agent.exe" --uninstall-service
& "C:\Program Files\TTGTiSO-Desk\ttgtiso-desk-agent.exe" --version
```

## Updating

```powershell
# Stops the service, swaps the binary for the latest release, restarts it
powershell -ExecutionPolicy Bypass -File update-agent.ps1
```

`update-agent.ps1` accepts the same `-Repo`, `-Version`, and `-BinaryPath`
parameters as the installer.

## Uninstalling

```powershell
powershell -ExecutionPolicy Bypass -File install-agent.ps1 -Uninstall
# or:
powershell -ExecutionPolicy Bypass -File uninstall-agent.ps1
```

This stops and removes the service. Program files and configuration are left in
place; delete `C:\Program Files\TTGTiSO-Desk` and `C:\ProgramData\TTGTiSO-Desk`
manually for a full removal.

## Firewall

The agent listens on the TCP `port` from `agent.toml` (default `2222`) for the
streaming protocol and broadcasts a small UDP discovery beacon on `9999`. Allow
the agent through Windows Defender Firewall if clients connect from other hosts:

```powershell
New-NetFirewallRule -DisplayName "TTGTiSO-Desk Agent" -Direction Inbound `
  -Action Allow -Protocol TCP -LocalPort 2222
```

## Troubleshooting

- **"This script must be run as Administrator"** — relaunch PowerShell elevated.
- **Service installed but not streaming** — confirm a user is logged in and the
  console session is unlocked; check `C:\ProgramData\TTGTiSO-Desk\logs\audit.log`.
- **Port already in use** — change `port` in `agent.toml` and restart the
  service.
