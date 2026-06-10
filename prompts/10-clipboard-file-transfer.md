# Prompt 10 — Clipboard and File Transfer

Implement clipboard sync and temporary file exchange.

## Clipboard Requirements

- bidirectional;
- text;
- images;
- HTML/RTF;
- file references through file-transfer channel;
- size limits;
- audit events;
- configurable policy.

## File Transfer Requirements

- temporary shared folder;
- SFTP or secure channel over SSH;
- resume support;
- hash verification;
- progress reporting;
- read/write policies;
- deny execution by default;
- offline network support.

## Required Crates

- `crates/clipboard`
- `crates/file-transfer`
- `crates/audit`

## Interfaces

Create:

- `ClipboardProvider`
- `ClipboardSyncService`
- `ClipboardPolicy`
- `FileTransferService`
- `TransferSession`
- `TransferPolicy`
- `HashVerifier`

## Tests

- text clipboard sync;
- size limit;
- file upload;
- file download;
- resume transfer;
- hash mismatch.
