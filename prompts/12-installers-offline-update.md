# Prompt 12 — Packaging, Installers and Offline Updates

Implement deployment support.

## Required Packages

Server:
- `.deb` for Astra Linux;
- shell installer;
- unattended install mode.

Client:
- `.exe`
- `.deb`
- `.rpm`
- AppImage

## Offline Requirements

- no internet required;
- offline bundle;
- local update repository option;
- GitHub update check optional only;
- proxy support optional.

## Server Install Paths

- config: `/etc/ttgtiso-desk/`
- data: `/var/lib/ttgtiso-desk/`
- logs: `/var/log/ttgtiso-desk/`
- binary: `/usr/bin/ttgtiso-desk-agent`

## Required Files

- systemd unit;
- default config;
- install script;
- uninstall script;
- hardening checklist;
- offline deployment guide.

## Documentation

Create:
- `docs/installation-astra.md`
- `docs/offline-deployment.md`
- `docs/update-strategy.md`
