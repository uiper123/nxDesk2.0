# Offline and Air-Gapped Deployment Guide

This guide details steps to assemble and execute offline installations of TTGTiSO-Desk in air-gapped environments.

---

## 1. Bundle Assembly

An offline deployment package consists of a compressed archive containing all dependencies pre-packaged:

```text
ttgtiso-desk-offline-bundle/
├── install-agent.sh              # Installation runner script
├── server-agent                   # Precompiled Astra Linux binary
├── templates/
│   ├── agent.toml.default         # Default config
│   └── ttgtiso-desk-agent.service # Systemd unit
└── dependencies/                  # Pre-packaged runtime dependencies
    ├── xvfb_*.deb                 # Virtual framebuffer X11
    └── fly-wm_*.deb               # Fly window manager packages
```

### Steps to compile the bundle:
1. Compile the server agent on a build machine matching the target Astra version:
   ```bash
   cargo build --release -p server-agent
   ```
2. Download required Debian packages with their recursive dependencies:
   ```bash
   mkdir dependencies
   apt-get download xvfb xauth x11-utils
   ```
3. Archive the structure:
   ```bash
   tar -czvf ttgtiso-desk-offline-bundle.tar.gz -C ttgtiso-desk-offline-bundle/ .
   ```

---

## 2. Target Air-Gapped Setup

Once the archive is transferred to the air-gapped server (e.g., via secure diode or encrypted USB storage):

1. Extract the bundle:
   ```bash
   tar -xzvf ttgtiso-desk-offline-bundle.tar.gz -C /tmp/bundle
   cd /tmp/bundle
   ```
2. Install the local Debian dependencies using `dpkg`:
   ```bash
   sudo dpkg -i dependencies/*.deb
   ```
3. Execute the unattended setup script:
   ```bash
   sudo ./install-agent.sh --unattended
   ```

---

## 3. Local Repository Config (Alternative)

If the target network hosts a private package repository (e.g., Nexus or local Apt mirror):

1. Configure `/etc/apt/sources.list.d/local.list` to point to the local server mirror:
   ```text
   deb http://repo.local/astra/ 1.7 main contrib non-free
   ```
2. Run standard APT installation from the local cache:
   ```bash
   sudo apt-get update
   sudo apt-get install xvfb fly-wm
   ```
