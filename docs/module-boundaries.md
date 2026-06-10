# Module Boundaries & API Interfaces — TTGTiSO-Desk

This document defines the clear boundaries, responsibilities, and APIs between the Rust modules inside `/crates` and packages inside `/packages`.

## 1. Module Responsibilities Matrix

| Crate | Responsibility | Dependencies | API Consumer |
| :--- | :--- | :--- | :--- |
| `shared-types` | Data transfer objects (DTOs) & shared structures | *None* | All crates, UI |
| `config` | Parsing & validation of `agent.toml` / client settings | `serde` | Agent, Client |
| `protocol` | Binary framing, encoding/decoding of packets (TTMP) | `shared-types` | Transport |
| `transport` | SSH tunneling, connection lifecycle, multiplexing | `protocol`, `russh` | Agent, Client |
| `security` | Authentication, RBAC checks, key management | `config` | Agent, Transport |
| `session-manager` | Spawning isolated X11 displays, lifecycle tracking | `os-pal`, `config` | Agent |
| `video-pipeline` | Capturing X11 display, encoding via GStreamer (H.264) | `shared-types`, `gstreamer` | Agent |
| `input-injector` | Injecting keyboard/mouse inputs via XTest | `shared-types`, `x11rb` | Agent |
| `clipboard` | Bi-directional text clipboard synchronization | `os-pal` | Agent, Client |
| `file-transfer` | Safe chunked file writes/reads & path sanitization | `shared-types` | Agent, Client |
| `audit` | Secure logging of all interactions to journald | `shared-types` | Agent, Security |
| `os-pal` | OS Platform Abstraction Layer (path resolvers, command runners) | *None* | Session-Manager |

---

## 2. Key Interface Definitions (Rust Traits)

To enforce loose coupling, modules interact via public traits (interfaces).

### 2.1. Session Manager Interface (`session-manager`)
```rust
pub trait SessionController {
    type Error;
    
    fn create_session(&self, username: &str, display_id: u8) -> Result<SessionInfo, Self::Error>;
    fn stop_session(&self, session_id: &str) -> Result<(), Self::Error>;
    fn get_session_status(&self, session_id: &str) -> Result<SessionStatus, Self::Error>;
    fn list_active_sessions(&self) -> Result<Vec<SessionInfo>, Self::Error>;
}
```

### 2.2. Video Pipeline Interface (`video-pipeline`)
```rust
pub trait VideoCapturer {
    type Error;
    
    fn start_capture(&self, display: &str, settings: VideoSettings) -> Result<FrameStream, Self::Error>;
    fn stop_capture(&self) -> Result<(), Self::Error>;
    fn adjust_bitrate(&self, bitrate_bps: u32) -> Result<(), Self::Error>;
}
```

### 2.3. Input Injector Interface (`input-injector`)
```rust
pub trait InputDevice {
    type Error;
    
    fn inject_mouse_move(&self, x: u16, y: u16) -> Result<(), Self::Error>;
    fn inject_mouse_click(&self, button: MouseButton, pressed: bool) -> Result<(), Self::Error>;
    fn inject_mouse_scroll(&self, delta: i16) -> Result<(), Self::Error>;
    fn inject_keypress(&self, keysym: u32, pressed: bool) -> Result<(), Self::Error>;
}
```

### 2.4. Audit Interface (`audit`)
```rust
pub trait AuditLogger {
    fn log_auth_success(&self, username: &str, ip: &str);
    fn log_auth_failure(&self, username: &str, ip: &str, reason: &str);
    fn log_session_start(&self, session_id: &str, username: &str, display: &str);
    fn log_session_stop(&self, session_id: &str, reason: &str);
    fn log_file_transfer(&self, path: &str, size: u64, is_upload: bool, success: bool);
}
```
