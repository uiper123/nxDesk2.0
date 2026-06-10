# Prompt 13 — Testing and CI

Implement the testing and CI strategy.

## Required Tests

- unit tests;
- integration tests;
- e2e tests;
- protocol tests;
- security tests;
- performance benchmarks;
- UI tests;
- mock server tests;
- Astra Linux manual/VM test plan.

## CI Requirements

Add CI jobs for:

- cargo fmt;
- cargo clippy;
- cargo test;
- npm install;
- npm lint;
- npm test;
- build desktop client;
- build server agent;
- security audit;
- artifact packaging skeleton.

## Required Documents

- `docs/testing.md`
- `docs/ci.md`
- `docs/astra-test-plan.md`

## MVP Test Scenario

The MVP is ready when:

1. server agent starts;
2. client connects securely;
3. user session is created;
4. video stream is shown;
5. mouse/keyboard input works;
6. clipboard text works;
7. file transfer works;
8. audit logs are written;
9. reconnect works;
10. everything works without internet.
