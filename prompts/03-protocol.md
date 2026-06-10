# Prompt 03 — TTGTiSO Multiplex Protocol

Design and implement the protocol layer.

## Goal

Create a custom multiplexed protocol over SSH transport.

## Channels

Implement protocol support for:

- control
- video
- input
- clipboard
- file
- audit
- heartbeat

## Requirements

- versioned protocol;
- binary framing;
- request/response messages;
- event messages;
- streaming messages;
- error codes;
- backpressure;
- heartbeat;
- reconnect support;
- capability negotiation.

## Required Crates

- `crates/protocol`
- `crates/transport`
- `crates/shared-types`

## Required Documents

- `docs/protocol.md`
- `docs/protocol-message-types.md`

## Testing

Add:
- serialization tests;
- deserialization tests;
- invalid frame tests;
- channel multiplexing tests;
- heartbeat tests.

## Rules

Interfaces first.
No file over 250 lines.
Use Rust types with `serde` where suitable.
Avoid transport-specific logic in the protocol crate.
