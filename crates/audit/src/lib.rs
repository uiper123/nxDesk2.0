use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    pub timestamp: u64,
    pub event_type: String,
    pub username: String,
    pub ip_address: String,
    pub action: String,
    pub details: String,
}

pub struct AuditLog {
    log_file_path: PathBuf,
    write_lock: Mutex<()>,
}

impl AuditLog {
    pub fn new(log_file_path: &Path) -> Self {
        // Ensure parent directory exists
        if let Some(parent) = log_file_path.parent() {
            let _ = create_dir_all(parent);
        }

        Self {
            log_file_path: log_file_path.to_path_buf(),
            write_lock: Mutex::new(()),
        }
    }

    pub fn write_record(&self, record: AuditRecord) {
        let serialized = serde_json::to_string(&record).unwrap_or_default();
        // Log to tracing as well
        tracing::info!(target: "audit", "{}", serialized);
        
        // Write to file
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
        {
            Ok(mut file) => {
                if let Err(e) = writeln!(file, "{}", serialized) {
                    tracing::warn!("Failed to write audit record to {:?}: {}", self.log_file_path, e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to open audit log file {:?}: {}", self.log_file_path, e);
            }
        }
    }

    pub fn log_auth_success(&self, username: &str, ip: &str) {
        self.write_record(AuditRecord {
            timestamp: now_secs(),
            event_type: "AUTHENTICATION".to_string(),
            username: username.to_string(),
            ip_address: ip.to_string(),
            action: "login_success".to_string(),
            details: format!("User {} successfully authenticated", username),
        });
    }

    pub fn log_auth_failure(&self, username: &str, ip: &str, reason: &str) {
        self.write_record(AuditRecord {
            timestamp: now_secs(),
            event_type: "AUTHENTICATION".to_string(),
            username: username.to_string(),
            ip_address: ip.to_string(),
            action: "login_failure".to_string(),
            details: format!("Failed authentication for {}: {}", username, reason),
        });
    }

    pub fn log_session_start(&self, session_id: &str, username: &str, display: &str) {
        self.write_record(AuditRecord {
            timestamp: now_secs(),
            event_type: "SESSION".to_string(),
            username: username.to_string(),
            ip_address: String::new(),
            action: "session_start".to_string(),
            details: format!("Session {} started on display {}", session_id, display),
        });
    }

    pub fn log_session_stop(&self, session_id: &str, username: &str, reason: &str) {
        self.write_record(AuditRecord {
            timestamp: now_secs(),
            event_type: "SESSION".to_string(),
            username: username.to_string(),
            ip_address: String::new(),
            action: "session_stop".to_string(),
            details: format!("Session {} stopped. Reason: {}", session_id, reason),
        });
    }

    /// Read the last N records from the audit log file
    pub fn read_last_records(&self, count: usize) -> Vec<AuditRecord> {
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        match std::fs::read_to_string(&self.log_file_path) {
            Ok(contents) => {
                contents.lines()
                    .rev()
                    .take(count)
                    .filter_map(|line| serde_json::from_str::<AuditRecord>(line).ok())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Return total number of records in the audit log
    pub fn record_count(&self) -> usize {
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        match std::fs::read_to_string(&self.log_file_path) {
            Ok(contents) => contents.lines().filter(|l| !l.trim().is_empty()).count(),
            Err(_) => 0,
        }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_record_formatting() {
        let temp_dir = std::env::temp_dir();
        let log_file = temp_dir.join("test_audit.log");
        let _audit = AuditLog::new(&log_file);
        
        let record = AuditRecord {
            timestamp: 1234567890,
            event_type: "TEST".to_string(),
            username: "vladimir".to_string(),
            ip_address: "127.0.0.1".to_string(),
            action: "test_action".to_string(),
            details: "testing audit subsystem".to_string(),
        };
        
        let serialized = serde_json::to_string(&record).unwrap();
        assert!(serialized.contains("\"username\":\"vladimir\""));
        assert!(serialized.contains("\"event_type\":\"TEST\""));
    }
}
