use anyhow::Result;
use shared_types::{SessionInfo, SessionStatus};

pub trait UserSession: Send + Sync {
    fn id(&self) -> &str;
    fn username(&self) -> &str;
    fn display_id(&self) -> u8;
    fn status(&self) -> SessionStatus;
    fn stop(&mut self) -> Result<()>;
}

pub trait SessionBackend: Send + Sync {
    fn create_session(&self, username: &str, display_id: u8) -> Result<Box<dyn UserSession>>;
}

pub trait DisplayAllocator: Send + Sync {
    fn allocate(&self) -> Result<u8>;
    fn release(&self, display_id: u8);
}

pub trait DesktopLauncher: Send + Sync {
    fn launch_desktop(&self, display_id: u8, username: &str) -> Result<std::process::Child>;
}

pub trait SessionLifecycle: Send + Sync {
    fn on_startup(&self) -> Result<()>;
    fn on_shutdown(&self) -> Result<()>;
}

pub trait SessionAuditSink: Send + Sync {
    fn log_start(&self, session_id: &str, username: &str, display_id: u8);
    fn log_stop(&self, session_id: &str, username: &str, reason: &str);
}

pub trait SessionManager: Send + Sync {
    fn start_session(&self, username: &str) -> Result<SessionInfo>;
    fn stop_session(&self, session_id: &str) -> Result<()>;
    fn get_session_info(&self, session_id: &str) -> Result<SessionInfo>;
    fn list_active_sessions(&self) -> Result<Vec<SessionInfo>>;
}
