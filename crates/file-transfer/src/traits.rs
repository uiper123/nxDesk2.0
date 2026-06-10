use anyhow::Result;

pub trait TransferSession: Send + Sync {
    fn id(&self) -> &str;
    fn file_name(&self) -> &str;
    fn file_size(&self) -> u64;
    fn bytes_transferred(&self) -> u64;
    fn is_completed(&self) -> bool;
}

pub trait TransferPolicy: Send + Sync {
    fn is_allowed(&self, file_name: &str, size: u64) -> bool;
    fn allow_execution(&self) -> bool;
}

pub trait HashVerifier: Send + Sync {
    fn verify(&self, data: &[u8], expected_hash: &str) -> bool;
    fn calculate_hash(&self, data: &[u8]) -> String;
}

pub trait FileTransferService: Send + Sync {
    fn start_upload(&self, file_name: &str, file_size: u64) -> Result<String>;
    fn upload_chunk(&self, session_id: &str, offset: u64, chunk: &[u8]) -> Result<()>;
    fn verify_upload(&self, session_id: &str, expected_hash: &str) -> Result<bool>;
}
