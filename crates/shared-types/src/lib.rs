use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Type, PartialEq, Eq)]
pub enum SessionKind {
    X11,
    Wayland,
    Virtual,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub enum AuthMethod {
    Password(String),
    PrivateKeyPath(String),
    Agent,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct SessionInfo {
    pub id: String,
    pub username: String,
    pub display_id: u8,
    pub start_time: u64,
    pub session_kind: SessionKind,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Type, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Idle,
    Disconnected,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct VideoSettings {
    pub width: u16,
    pub height: u16,
    pub fps: u8,
    pub target_bitrate_kbps: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type, PartialEq, Eq)]
pub enum MouseButton {
    None,
    Left,
    Right,
    Middle,
}
