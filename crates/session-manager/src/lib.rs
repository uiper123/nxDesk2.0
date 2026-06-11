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
    /// Creates a session manager using the real Astra X11 backend.
    /// Falls back to MockSessionBackend if Xvfb is not available.
    pub fn new_default() -> Self {
        let backend: Box<dyn SessionBackend> = if cfg!(target_os = "linux") {
            // Try to detect if Xvfb is available
            let xvfb_available = std::process::Command::new("which")
                .arg("Xvfb")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if xvfb_available {
                tracing::info!("Xvfb detected, using AstraX11Backend for sessions");
                Box::new(AstraX11Backend)
            } else {
                tracing::warn!("Xvfb not found, using MockSessionBackend for sessions");
                Box::new(MockSessionBackend)
            }
        } else {
            tracing::info!("Non-Linux OS detected, using MockSessionBackend");
            Box::new(MockSessionBackend)
        };

        let allocator = std::sync::Arc::new(LocalDisplayAllocator::new(10, 99));
        Self::new(backend, allocator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use traits::{DisplayAllocator, SessionManager};

    #[test]
    fn test_display_allocation() {
        let allocator = LocalDisplayAllocator::new(10, 12);

        let d1 = allocator.allocate().unwrap();
        let d2 = allocator.allocate().unwrap();
        let d3 = allocator.allocate().unwrap();

        assert_eq!(d1, 10);
        assert_eq!(d2, 11);
        assert_eq!(d3, 12);

        // Next allocation should fail since we only configured 10-12
        assert!(allocator.allocate().is_err());

        allocator.release(d2);
        let d4 = allocator.allocate().unwrap();
        assert_eq!(d4, 11);
    }

    #[test]
    fn test_session_manager_lifecycle() {
        let manager = LocalSessionManager::new_default();

        // Start session
        let info = manager.start_session("pavel").unwrap();
        assert_eq!(info.username, "pavel");
        assert_eq!(info.display_id, 10);

        // Try duplicate session for the same user (should fail)
        assert!(manager.start_session("pavel").is_err());

        // Stop session
        let stop_res = manager.stop_session(&info.id);
        assert!(stop_res.is_ok());

        // Try stopping again (should fail)
        assert!(manager.stop_session(&info.id).is_err());
    }
}
