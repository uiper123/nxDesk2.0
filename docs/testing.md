# Testing Strategy and Architecture

This document describes the testing strategy, test classifications, and verification guidelines for the TTGTiSO-Desk Remote Desktop system.

---

## 1. Test Classifications

The project utilizes a multi-layered testing pyramid to validate system components at every tier:

### 1.1. Unit Tests
* Located inline or within `mod tests` block inside each workspace crate (e.g. `crates/shared-types`, `crates/session-manager`, `crates/video-pipeline`).
* Focused on stateless functions, encoding/decoding, policy rule evaluation, and displays allocation logic.
* **To run all Rust unit tests**:
  ```bash
  cargo test --lib
  ```

### 1.2. Integration Tests
* Validate inter-crate operations and network sockets.
* Examples:
  * `crates/file-transfer` tests verifying chunk assembly, resume support, and checksum verifiers.
  * `apps/relay-server` tests connecting mock agents and clients to verify full-duplex TCP routing, heartbeat timeouts, and token authentication.
* **To run all Rust integration tests**:
  ```bash
  cargo test --bins
  ```

### 1.3. Protocol Tests
* Located in `crates/protocol/src/lib.rs`.
* Validates binary packet layouts, custom framing, magic signature byte constraints, and serialization schemas for custom mouse, keyboard, and file exchange control messages.

### 1.4. Security and Policy Tests
* Verify that RBAC constraints, restricted keystroke filters (e.g., VT-switching Ctrl+Alt+F* blocking), and file transfer extension lists correctly deny invalid interactions.
* Located in:
  * `crates/security/src/lib.rs` (RBAC, credentials logs scrubbing).
  * `crates/input-injector/src/policy.rs` (Forbidden key injection blocks).
  * `crates/file-transfer/src/policy.rs` (Blocked extension verification).

---

## 2. Performance Benchmarks

* Validate frame Clock pacing stability (ensuring consistent 30 FPS under load) and AIMD latency calculations for Adaptive Bitrate Control.
* Located in `crates/video-pipeline/src/lib.rs` (`test_perf_benchmark_skeleton`).

---

## 3. UI and Mock Server Verification

### Frontend Unit & UI Tests
* UI mock verification and component structures can be validated locally via Vite/React unit testing frameworks.
* All component interactions, layout panels, and login flows can be tested with Jest or Vitest.

---

## 4. Run Commands Reference

To run the entire test suite locally:

```bash
# Run all Rust tests (unit, integration, protocol, security)
cargo test

# Run frontend tests
cd apps/desktop-client
npm run test
```
