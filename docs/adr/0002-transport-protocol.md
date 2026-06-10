# ADR 0002: Transport Protocol Selection

## Status
Accepted

## Context
The system must operate in closed local networks under strict firewall regulations. Often, only a single port (typically SSH port 22) is allowed for inbound connections. We need a secure transport that supports low-latency video streaming, mouse/keyboard inputs, file transfers, and clipboard synchronization.

## Considered Alternatives
1. **Raw TCP/UDP Sockets with TLS:** Requires setting up a local PKI (Public Key Infrastructure) to manage certificates, and requires opening multiple ports or designing a custom multiplexer over TLS.
2. **WebRTC:** Excellent performance and native latency controls. However, it requires a complex signaling server, STUN/TURN servers (difficult in offline closed networks), and does not easily tunnel through a single SSH port without complex gateways.
3. **SSH with Custom Multiplexed Protocol (TTMP):** Connects to the server's SSH daemon, authenticates using native SSH keys or PAM, opens a single channel, and runs a custom binary multiplexing framing protocol (TTMP) over that channel.

## Decision
We choose **SSH as the transport layer**, encapsulating our **TTMP (TTGTiSO Multiplex Protocol)**:
- Utilize native or embedded SSH client configurations in Rust (via `russh` or `ssh2`).
- The client establishes an SSH connection to the server agent.
- Inside the SSH session, the client opens a subsystem or executes the agent CLI to start the binary multiplexer stream.
- All subsequent logical channels (video, input, clipboard, file transfer, audit) are framed and multiplexed within this single stream.

## Consequences
- **Pros:**
  - Firewalls only need to permit standard SSH port 22 traffic.
  - Zero-configuration security: leverages existing SSH infrastructure, host key verification, and secure key exchanges.
  - User authentication and authorization align natively with target server OS users.
- **Cons:**
  - SSH runs over TCP, which introduces potential head-of-line blocking under packet loss compared to UDP-based protocols (like WebRTC).
  - Encapsulating video inside SSH adds standard TCP windowing and encryption overhead, requiring optimized framing.
