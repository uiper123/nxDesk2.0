pub mod policy;
pub mod sync;
pub mod traits;

pub use policy::ConfigurableClipboardPolicy;
pub use sync::DefaultClipboardSyncService;
pub use traits::{ClipboardContent, ClipboardPolicy, ClipboardProvider, ClipboardSyncService};

use anyhow::Result;
use std::sync::Mutex;

// Mock provider for testing
pub struct MockClipboardProvider {
    store: Mutex<Option<ClipboardContent>>,
}

impl MockClipboardProvider {
    pub fn new(initial: Option<ClipboardContent>) -> Self {
        Self {
            store: Mutex::new(initial),
        }
    }
}

impl ClipboardProvider for MockClipboardProvider {
    fn read(&self) -> Result<Option<ClipboardContent>> {
        Ok(self.store.lock().unwrap().clone())
    }

    fn write(&self, content: ClipboardContent) -> Result<()> {
        *self.store.lock().unwrap() = Some(content);
        Ok(())
    }
}

use std::io::Write;
use std::process::{Command, Stdio};

pub struct X11ClipboardProvider {
    display: String,
    fallback_store: Mutex<Option<ClipboardContent>>,
}

impl X11ClipboardProvider {
    pub fn new(display: &str) -> Self {
        Self {
            display: display.to_string(),
            fallback_store: Mutex::new(None),
        }
    }

    fn has_xclip(&self) -> bool {
        Command::new("which")
            .arg("xclip")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

impl ClipboardProvider for X11ClipboardProvider {
    fn read(&self) -> Result<Option<ClipboardContent>> {
        if !self.has_xclip() {
            tracing::warn!("xclip not found. Falling back to local mock clipboard memory.");
            return Ok(self.fallback_store.lock().unwrap().clone());
        }

        let output = Command::new("xclip")
            .env("DISPLAY", &self.display)
            .arg("-selection")
            .arg("clipboard")
            .arg("-o")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout).into_owned();
                if text.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(ClipboardContent::Text(text)))
                }
            }
            _ => Ok(self.fallback_store.lock().unwrap().clone()),
        }
    }

    fn write(&self, content: ClipboardContent) -> Result<()> {
        *self.fallback_store.lock().unwrap() = Some(content.clone());

        if !self.has_xclip() {
            return Ok(());
        }

        if let ClipboardContent::Text(text) = content {
            let mut child = Command::new("xclip")
                .env("DISPLAY", &self.display)
                .arg("-selection")
                .arg("clipboard")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            let _ = child.wait();
        }
        Ok(())
    }
}

// Legacy structure compatibility wrapper
pub struct ClipboardSync {
    _display: String,
}

impl ClipboardSync {
    pub fn new(display: &str) -> Self {
        Self {
            _display: display.to_string(),
        }
    }

    pub fn get_text(&self) -> Result<Option<String>> {
        let provider = X11ClipboardProvider::new(&self._display);
        match provider.read()? {
            Some(ClipboardContent::Text(t)) => Ok(Some(t)),
            _ => Ok(None),
        }
    }

    pub fn set_text(&self, text: &str) -> Result<()> {
        let provider = X11ClipboardProvider::new(&self._display);
        provider.write(ClipboardContent::Text(text.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audit::AuditLog;
    use std::sync::Arc;

    #[test]
    fn test_text_clipboard_sync() {
        let policy = Arc::new(ConfigurableClipboardPolicy::new_default());
        let temp_dir = std::env::temp_dir();
        let audit = Arc::new(AuditLog::new(&temp_dir.join("test_clipboard_audit.log")));
        let sync_service = DefaultClipboardSyncService::new(policy, audit);

        let local_provider =
            MockClipboardProvider::new(Some(ClipboardContent::Text("Hello, Astra!".to_string())));
        let remote_provider = MockClipboardProvider::new(None);

        // Sync local to remote
        assert!(sync_service
            .sync_to_remote(&local_provider, &remote_provider)
            .is_ok());

        // Read from remote
        let remote_content = remote_provider.read().unwrap();
        assert_eq!(
            remote_content,
            Some(ClipboardContent::Text("Hello, Astra!".to_string()))
        );
    }

    #[test]
    fn test_clipboard_size_limit() {
        let mut policy = ConfigurableClipboardPolicy::new_default();
        policy.max_text_len = 10; // set small limit for testing
        let policy_arc = Arc::new(policy);

        let temp_dir = std::env::temp_dir();
        let audit = Arc::new(AuditLog::new(&temp_dir.join("test_clipboard_size.log")));
        let sync_service = DefaultClipboardSyncService::new(policy_arc, audit);

        // Content fits limit (7 chars)
        let local_ok =
            MockClipboardProvider::new(Some(ClipboardContent::Text("1234567".to_string())));
        let remote_dest = MockClipboardProvider::new(None);
        assert!(sync_service.sync_to_remote(&local_ok, &remote_dest).is_ok());

        // Content exceeds limit (11 chars)
        let local_bad =
            MockClipboardProvider::new(Some(ClipboardContent::Text("12345678901".to_string())));
        let remote_dest_bad = MockClipboardProvider::new(None);
        assert!(sync_service
            .sync_to_remote(&local_bad, &remote_dest_bad)
            .is_err());
    }
}
