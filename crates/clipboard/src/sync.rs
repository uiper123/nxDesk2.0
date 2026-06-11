use crate::traits::{ClipboardPolicy, ClipboardProvider, ClipboardSyncService};
use anyhow::{bail, Result};
use audit::{AuditLog, AuditRecord};
use std::sync::Arc;

pub struct DefaultClipboardSyncService {
    policy: Arc<dyn ClipboardPolicy>,
    audit: Arc<AuditLog>,
}

impl DefaultClipboardSyncService {
    pub fn new(policy: Arc<dyn ClipboardPolicy>, audit: Arc<AuditLog>) -> Self {
        Self { policy, audit }
    }

    fn perform_sync(
        &self,
        direction: &str,
        source: &dyn ClipboardProvider,
        destination: &dyn ClipboardProvider,
    ) -> Result<()> {
        if let Some(content) = source.read()? {
            if self.policy.is_allowed(&content) {
                destination.write(content.clone())?;

                self.audit.write_record(AuditRecord {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    event_type: "CLIPBOARD".to_string(),
                    username: "system_operator".to_string(),
                    ip_address: "127.0.0.1".to_string(),
                    action: format!("sync_{}", direction),
                    details: format!("Synced clipboard content successfully. Type: {:?}", content),
                });
            } else {
                self.audit.write_record(AuditRecord {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    event_type: "CLIPBOARD_VIOLATION".to_string(),
                    username: "system_operator".to_string(),
                    ip_address: "127.0.0.1".to_string(),
                    action: format!("sync_rejected_{}", direction),
                    details: format!(
                        "Blocked clipboard sync due to policy violation. Content type: {:?}",
                        content
                    ),
                });
                bail!("Clipboard sync rejected: policy violation");
            }
        }
        Ok(())
    }
}

impl ClipboardSyncService for DefaultClipboardSyncService {
    fn sync_to_remote(
        &self,
        local: &dyn ClipboardProvider,
        remote: &dyn ClipboardProvider,
    ) -> Result<()> {
        self.perform_sync("local_to_remote", local, remote)
    }

    fn sync_to_local(
        &self,
        remote: &dyn ClipboardProvider,
        local: &dyn ClipboardProvider,
    ) -> Result<()> {
        self.perform_sync("remote_to_local", remote, local)
    }
}
