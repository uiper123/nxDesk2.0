use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub user: Option<UserInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub username: String,
    pub role: String,
    pub token: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub status: HostStatus,
    pub active_sessions: u32,
    pub operating_system: String,
    pub ssh_public_key: Option<String>,
    pub ssh_public_key_path: Option<String>,
    pub ssh_private_key_path: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HostStatus {
    Online,
    Offline,
    Busy,
}

#[derive(Debug, Serialize, Clone)]
pub struct ActiveSession {
    pub id: String,
    pub username: String,
    pub display_id: u32,
    pub start_time: String,
    pub cpu_usage: f32,
    pub mem_usage: u32,
    pub host_ip: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Audit,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub quality: String,
    pub encoder: String,
    pub fps: u32,
    pub audio: bool,
}
