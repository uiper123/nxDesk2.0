use crate::traits::{FileTransferService, HashVerifier, TransferPolicy};
use anyhow::{bail, Result};
use audit::{AuditLog, AuditRecord};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct FileSessionState {
    _id: String,
    file_name: String,
    file_size: u64,
    data: Vec<u8>,
}

pub struct DefaultFileTransferService {
    policy: Arc<dyn TransferPolicy>,
    verifier: Arc<dyn HashVerifier>,
    audit: Arc<AuditLog>,
    sessions: Mutex<HashMap<String, FileSessionState>>,
}

impl DefaultFileTransferService {
    pub fn new(
        policy: Arc<dyn TransferPolicy>,
        verifier: Arc<dyn HashVerifier>,
        audit: Arc<AuditLog>,
    ) -> Self {
        Self {
            policy,
            verifier,
            audit,
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl FileTransferService for DefaultFileTransferService {
    fn start_upload(&self, file_name: &str, file_size: u64) -> Result<String> {
        if !self.policy.is_allowed(file_name, file_size) {
            self.audit.write_record(AuditRecord {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                event_type: "FILE_TRANSFER_VIOLATION".to_string(),
                username: "system_operator".to_string(),
                ip_address: "127.0.0.1".to_string(),
                action: "upload_blocked".to_string(),
                details: format!("Blocked file upload: {} ({} bytes)", file_name, file_size),
            });
            bail!("File upload blocked by security policy");
        }

        let session_id = format!("tx-{}", uuid_simple());

        let state = FileSessionState {
            _id: session_id.clone(),
            file_name: file_name.to_string(),
            file_size,
            data: Vec::new(),
        };

        self.sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), state);

        self.audit.write_record(AuditRecord {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event_type: "FILE_TRANSFER".to_string(),
            username: "system_operator".to_string(),
            ip_address: "127.0.0.1".to_string(),
            action: "upload_start".to_string(),
            details: format!(
                "Started upload session: {} for file {}",
                session_id, file_name
            ),
        });

        Ok(session_id)
    }

    fn upload_chunk(&self, session_id: &str, offset: u64, chunk: &[u8]) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let state = match sessions.get_mut(session_id) {
            Some(s) => s,
            None => bail!("Active file transfer session not found"),
        };

        // Enforce boundary safety
        if offset + chunk.len() as u64 > state.file_size {
            bail!("Chunk writing bounds check violation");
        }

        // Auto-expand/pad buffer to handle offsets (supporting resume/out-of-order)
        if state.data.len() < (offset + chunk.len() as u64) as usize {
            state.data.resize((offset + chunk.len() as u64) as usize, 0);
        }

        // Copy chunk data into session buffer
        let start = offset as usize;
        let end = start + chunk.len();
        state.data[start..end].copy_from_slice(chunk);

        Ok(())
    }

    fn verify_upload(&self, session_id: &str, expected_hash: &str) -> Result<bool> {
        let mut sessions = self.sessions.lock().unwrap();
        let state = match sessions.remove(session_id) {
            Some(s) => s,
            None => bail!("Active file transfer session not found"),
        };

        if state.data.len() as u64 != state.file_size {
            bail!("Incomplete file upload buffer length mismatch");
        }

        let verified = self.verifier.verify(&state.data, expected_hash);

        self.audit.write_record(AuditRecord {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event_type: "FILE_TRANSFER".to_string(),
            username: "system_operator".to_string(),
            ip_address: "127.0.0.1".to_string(),
            action: if verified {
                "upload_success".to_string()
            } else {
                "upload_hash_mismatch".to_string()
            },
            details: format!(
                "Verify file {}: verified={}. Length={}",
                state.file_name,
                verified,
                state.data.len()
            ),
        });

        Ok(verified)
    }
}

fn uuid_simple() -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    nanos.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
