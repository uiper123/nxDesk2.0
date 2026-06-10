use crate::traits::TransferPolicy;

pub struct SecureTransferPolicy {
    pub max_file_size: u64,
    pub blocked_extensions: Vec<String>,
}

impl SecureTransferPolicy {
    pub fn new_default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024, // 100 MB
            blocked_extensions: vec![
                ".sh".to_string(),
                ".exe".to_string(),
                ".bin".to_string(),
                ".elf".to_string(),
                ".bat".to_string(),
            ],
        }
    }
}

impl TransferPolicy for SecureTransferPolicy {
    fn is_allowed(&self, file_name: &str, size: u64) -> bool {
        if size > self.max_file_size {
            return false;
        }

        let lower = file_name.to_lowercase();
        for ext in &self.blocked_extensions {
            if lower.ends_with(ext) {
                return false;
            }
        }
        true
    }

    fn allow_execution(&self) -> bool {
        false // Deny execution by default (strict security net baseline)
    }
}
