# Prompt 14 — Production Hardening

Prepare TTGTiSO-Desk for production-like deployment.

## Goals

- secure defaults;
- stable service behavior;
- crash recovery;
- auditability;
- maintainability;
- readiness for future Astra Linux K2 attestation preparation.

## Required Work

- config validation;
- least privilege service user;
- systemd hardening;
- log rotation;
- crash recovery;
- watchdog;
- safe update strategy;
- secure file permissions;
- dependency audit;
- known limitations document;
- admin troubleshooting guide.

## Required Documents

- `docs/hardening.md`
- `docs/production-checklist.md`
- `docs/troubleshooting.md`
- `docs/known-limitations.md`
- `docs/attestation-preparation-notes.md`

## Definition of Done

- service runs with minimal privileges;
- configs have safe defaults;
- logs are structured;
- failures are recoverable;
- deployment is documented;
- security checklist is complete.
