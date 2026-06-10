use anyhow::Result;
use shared_types::SessionStatus;
use crate::traits::{SessionBackend, UserSession};

pub struct MockUserSession {
    id: String,
    username: String,
    display_id: u8,
    status: SessionStatus,
}

impl MockUserSession {
    pub fn new(username: &str, display_id: u8) -> Self {
        Self {
            id: format!("{}-{}", username, display_id),
            username: username.to_string(),
            display_id,
            status: SessionStatus::Active,
        }
    }
}

impl UserSession for MockUserSession {
    fn id(&self) -> &str {
        &self.id
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn display_id(&self) -> u8 {
        self.display_id
    }

    fn status(&self) -> SessionStatus {
        self.status
    }

    fn stop(&mut self) -> Result<()> {
        self.status = SessionStatus::Disconnected;
        Ok(())
    }
}

pub struct MockSessionBackend;

impl SessionBackend for MockSessionBackend {
    fn create_session(&self, username: &str, display_id: u8) -> Result<Box<dyn UserSession>> {
        Ok(Box::new(MockUserSession::new(username, display_id)))
    }
}
