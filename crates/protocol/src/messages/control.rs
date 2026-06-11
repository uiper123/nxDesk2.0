use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ControlMessage {
    ClientHello(ClientHello),
    ServerHello(ServerHello),
    StartSessionRequest(StartSessionRequest),
    StopSessionRequest,
    SessionStarted(SessionStarted),
    ErrorMessage(ErrorMessage),
    // --- Wave 3 additions (backward compatible: ignored by older peers) ---
    /// Client asks the agent for the list of physical monitors it can stream.
    ListMonitorsRequest,
    /// Agent's reply enumerating the available monitors.
    MonitorList(MonitorList),
    /// Client asks to switch the active stream to a specific monitor.
    SelectMonitor(SelectMonitor),
    /// Client requests interactive access; the agent may prompt the local user.
    AccessRequest(AccessRequest),
    /// Agent's decision on an access request.
    AccessDecision(AccessDecision),
    /// Client offers a resume token to re-attach to a frozen session after a
    /// transient network drop.
    ResumeSessionRequest(ResumeSessionRequest),
    /// Agent's reply to a resume attempt.
    ResumeSessionResponse(ResumeSessionResponse),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ClientHello {
    pub client_version: String,
    pub supported_capabilities: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ServerHello {
    pub server_version: String,
    pub negotiated_capabilities: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StartSessionRequest {
    pub width: u16,
    pub height: u16,
    pub fps: u8,
    /// Index of the monitor to stream. Defaults to 0 (primary) for legacy
    /// clients that don't send it.
    #[serde(default)]
    pub monitor_index: u32,
    /// Preferred video codec capability token (e.g. "video.h264"); empty means
    /// "let the agent decide / legacy PNG".
    #[serde(default)]
    pub preferred_codec: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SessionStarted {
    pub session_id: String,
    pub display: String,
    /// Opaque token the client can present later to resume this session after a
    /// transient disconnect. Empty if the agent does not support resume.
    #[serde(default)]
    pub resume_token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ErrorMessage {
    pub code: u16,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Multi-monitor
// ---------------------------------------------------------------------------

/// A single physical monitor exposed by the host.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    /// Zero-based index; 0 is conventionally the primary monitor.
    pub index: u32,
    /// Human-friendly label (connector name / model when available).
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Virtual-desktop offset of this monitor's top-left corner.
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MonitorList {
    pub monitors: Vec<MonitorInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SelectMonitor {
    pub index: u32,
}

// ---------------------------------------------------------------------------
// Access control: unattended vs. "ask the local user"
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// No prompt: the agent grants access based on policy/credentials alone.
    Unattended,
    /// The local user at the console must approve the incoming connection.
    AskUser,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AccessRequest {
    /// Display name of the operator requesting access (shown in the prompt).
    pub operator: String,
    /// Free-text reason shown to the local user.
    #[serde(default)]
    pub reason: String,
    pub mode: AccessMode,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AccessDecision {
    pub granted: bool,
    /// Populated when `granted` is false.
    #[serde(default)]
    pub message: String,
}

// ---------------------------------------------------------------------------
// Session resume (network hiccup tolerance)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ResumeSessionRequest {
    pub resume_token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ResumeSessionResponse {
    pub resumed: bool,
    /// Echoes the session id on success; empty on failure.
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub message: String,
}
