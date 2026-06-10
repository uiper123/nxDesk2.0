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
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SessionStarted {
    pub session_id: String,
    pub display: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ErrorMessage {
    pub code: u16,
    pub message: String,
}
