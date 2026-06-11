pub mod policy;
pub mod service;
pub mod traits;
pub mod verifier;

pub use policy::SecureTransferPolicy;
pub use service::DefaultFileTransferService;
pub use traits::{FileTransferService, HashVerifier, TransferPolicy, TransferSession};
pub use verifier::DefaultHashVerifier;

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

// Legacy compatibility struct
pub struct FileTransferManager {
    base_dir: PathBuf,
}

impl FileTransferManager {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    pub fn write_chunk(&self, relative_path: &str, offset: u64, data: &[u8]) -> Result<()> {
        let target_path = self.base_dir.join(relative_path);
        if !target_path.starts_with(&self.base_dir) {
            bail!("Unauthorized path access attempt");
        }
        tracing::debug!(
            "Writing {} bytes to {:?} at offset {}",
            data.len(),
            target_path,
            offset
        );
        Ok(())
    }

    pub fn read_chunk(&self, relative_path: &str, offset: u64, size: usize) -> Result<Vec<u8>> {
        let target_path = self.base_dir.join(relative_path);
        if !target_path.starts_with(&self.base_dir) {
            bail!("Unauthorized path access attempt");
        }
        tracing::debug!(
            "Reading up to {} bytes from {:?} at offset {}",
            size,
            target_path,
            offset
        );
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audit::AuditLog;
    use std::sync::Arc;

    #[test]
    fn test_file_upload_and_verify() {
        let policy = Arc::new(SecureTransferPolicy::new_default());
        let verifier = Arc::new(DefaultHashVerifier);
        let temp_dir = std::env::temp_dir();
        let audit = Arc::new(AuditLog::new(&temp_dir.join("test_upload.log")));
        let service = DefaultFileTransferService::new(policy, verifier.clone(), audit);

        // Start upload
        let session_id = service.start_upload("document.txt", 12).unwrap();

        // Upload chunks
        service.upload_chunk(&session_id, 0, b"Hello ").unwrap();
        service.upload_chunk(&session_id, 6, b"World!").unwrap();

        // Calculate expected hash
        let expected_hash = verifier.calculate_hash(b"Hello World!");

        // Verify upload
        let is_ok = service.verify_upload(&session_id, &expected_hash).unwrap();
        assert!(is_ok);
    }

    #[test]
    fn test_file_upload_hash_mismatch() {
        let policy = Arc::new(SecureTransferPolicy::new_default());
        let verifier = Arc::new(DefaultHashVerifier);
        let temp_dir = std::env::temp_dir();
        let audit = Arc::new(AuditLog::new(&temp_dir.join("test_mismatch.log")));
        let service = DefaultFileTransferService::new(policy, verifier, audit);

        let session_id = service.start_upload("data.csv", 5).unwrap();
        service.upload_chunk(&session_id, 0, b"rules").unwrap();

        // Verify against bad hash
        let is_ok = service.verify_upload(&session_id, "wronghash123").unwrap();
        assert!(!is_ok);
    }

    #[test]
    fn test_resume_out_of_order() {
        let policy = Arc::new(SecureTransferPolicy::new_default());
        let verifier = Arc::new(DefaultHashVerifier);
        let temp_dir = std::env::temp_dir();
        let audit = Arc::new(AuditLog::new(&temp_dir.join("test_resume.log")));
        let service = DefaultFileTransferService::new(policy, verifier.clone(), audit);

        let session_id = service.start_upload("resume.dat", 10).unwrap();

        // Upload second chunk first
        service.upload_chunk(&session_id, 5, b"world").unwrap();
        // Upload first chunk second
        service.upload_chunk(&session_id, 0, b"hello").unwrap();

        let expected_hash = verifier.calculate_hash(b"helloworld");
        let is_ok = service.verify_upload(&session_id, &expected_hash).unwrap();
        assert!(is_ok);
    }

    #[test]
    fn test_blocked_extension() {
        let policy = Arc::new(SecureTransferPolicy::new_default());
        let verifier = Arc::new(DefaultHashVerifier);
        let temp_dir = std::env::temp_dir();
        let audit = Arc::new(AuditLog::new(&temp_dir.join("test_blocked.log")));
        let service = DefaultFileTransferService::new(policy, verifier, audit);

        // Blocked extension .sh
        let result = service.start_upload("malicious.sh", 100);
        assert!(result.is_err());
    }
}
