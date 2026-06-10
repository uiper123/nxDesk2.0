# Production Hardening and Security Guidelines

This document details the hardening configurations, sandboxing policies, and security mechanisms deployed to secure the TTGTiSO-Desk Remote Desktop Server Agent.

---

## 1. Systemd Service Sandboxing

To enforce the principle of least privilege, the systemd unit file incorporates Linux kernel namespaces and security capabilities:

* **`ProtectSystem=full`**: Mounts `/usr`, `/boot`, and `/etc` as read-only.
* **`ProtectHome=read-only`**: Prevents the agent from altering files inside user home folders.
* **`PrivateTmp=true`**: Instantiates an isolated `/tmp` workspace namespace for the agent, invisible to other system processes.
* **`NoNewPrivileges=true`**: Prevents the agent process and its children from gaining new privileges via `setuid` or `setgid` flags.
* **`CapabilityBoundingSet`**: Restricts the process capabilities to:
  * `CAP_NET_BIND_SERVICE`: to listen on port 22 (SSH).
  * `CAP_SYS_ADMIN`: to manage display namespaces.
  * `CAP_SETUID` / `CAP_SETGID`: to switch user contexts during isolated session starts.

---

## 2. Directory and File Level Access Controls

Strict DAC (Discretionary Access Control) permissions are enforced during deployment:

```text
/etc/ttgtiso-desk/            -> Owner root:root, Mode 750
/etc/ttgtiso-desk/agent.toml  -> Owner root:root, Mode 600
/var/lib/ttgtiso-desk/        -> Owner root:root, Mode 700
/var/log/ttgtiso-desk/        -> Owner root:root, Mode 700
```

---

## 3. Log Rotation Policy

To avoid Denial-of-Service scenarios via disk space exhaustion, configure log rotation for the audit journal at `/etc/logrotate.d/ttgtiso-desk`:

```text
/var/log/ttgtiso-desk/*.log {
    daily
    rotate 14
    compress
    delaycompress
    missingok
    notifempty
    create 0600 root root
    sharedscripts
    postrotate
        systemctl kill -s HUP ttgtiso-desk-agent.service >/dev/null 2>&1 || true
    endscript
}
```

---

## 4. Secure Default Configuration

The default configuration file restricts access by default:
* Password-less SSH auth is disabled unless explicit keys are registered.
* Execution of scripts through file transfer is blocked.
* Heartbeat watchdog checks run every second to clear stale allocations.
