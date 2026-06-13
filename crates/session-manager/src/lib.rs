pub mod allocator;
pub mod backends;
pub mod continuity;
pub mod manager;
pub mod traits;

pub use allocator::LocalDisplayAllocator;
pub use backends::astra_x11::AstraX11Backend;
pub use backends::mock::MockSessionBackend;
pub use continuity::{ResumeRegistry, ResumeState, DEFAULT_GRACE};
pub use manager::LocalSessionManager;
pub use traits::{
    DisplayAllocator, SessionAuditSink, SessionBackend, SessionLifecycle, SessionManager,
    UserSession,
};

impl LocalSessionManager {
    pub fn from_config(cfg: &config::AgentConfig) -> Self {
        let backend: Box<dyn SessionBackend> = if cfg!(target_os = "linux") {
            Box::new(AstraX11Backend::from_config(
                cfg.connection_mode.clone(),
                cfg.desktop_session_type.clone(),
            ))
        } else {
            Box::new(MockSessionBackend)
        };

        let allocator = std::sync::Arc::new(LocalDisplayAllocator::new(10, 99));
        Self::new(backend, allocator)
    }

    pub fn new_default() -> Self {
        Self::from_config(&config::AgentConfig::default())
    }
}
