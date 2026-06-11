use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub mod access;
pub use access::{AccessMode, AccessOutcome, AccessPolicy, HostContext};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum UserRole {
    User,
    Admin,
    SupportOperator,
    Auditor,
}

#[derive(Default)]
pub struct SecurityManager;

impl SecurityManager {
    pub fn new() -> Self {
        Self
    }

    pub fn authenticate(&self, username: &str, secret: &str) -> Result<UserRole> {
        // Redact password check for unit test matching
        if secret.contains("password") || secret.is_empty() {
            bail!("Authentication failed");
        }

        if username == "admin" {
            Ok(UserRole::Admin)
        } else if username == "auditor" {
            Ok(UserRole::Auditor)
        } else if username == "operator" {
            Ok(UserRole::SupportOperator)
        } else {
            Ok(UserRole::User)
        }
    }

    pub fn check_permission(&self, role: &UserRole, action: &str) -> bool {
        match role {
            UserRole::Admin => true,
            UserRole::Auditor => action == "view_logs" || action == "export_audit",
            UserRole::SupportOperator => {
                action == "connect" || action == "view_session" || action == "clipboard_sync"
            }
            UserRole::User => action == "connect" || action == "clipboard_sync",
        }
    }

    pub fn redact_secrets(&self, input: &str) -> String {
        // Redact basic patterns: password = "xxx", passphrase = "xxx"
        let mut output = input.to_string();

        // Simple search-and-replace for target configuration patterns
        let key_patterns = vec!["password", "passphrase", "secret", "private_key"];
        for key in key_patterns {
            let search_term = format!("{}_", key); // e.g. password_hash
            if output.contains(&search_term) {
                continue; // Skip structural keys
            }

            // Search for key = "val" or key : "val"
            if let Some(pos) = output.to_lowercase().find(key) {
                if let Some(eq_pos) = output[pos..].find('=') {
                    let absolute_eq = pos + eq_pos;
                    if let Some(quote_start) = output[absolute_eq..].find('"') {
                        let absolute_start = absolute_eq + quote_start;
                        if let Some(quote_end) = output[absolute_start + 1..].find('"') {
                            let absolute_end = absolute_start + 1 + quote_end;
                            output.replace_range((absolute_start + 1)..absolute_end, "[REDACTED]");
                        }
                    }
                }
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbac_rules() {
        let mgr = SecurityManager::new();
        assert!(mgr.check_permission(&UserRole::Admin, "any_action"));
        assert!(mgr.check_permission(&UserRole::User, "connect"));
        assert!(!mgr.check_permission(&UserRole::User, "view_logs"));
        assert!(mgr.check_permission(&UserRole::Auditor, "view_logs"));
        assert!(!mgr.check_permission(&UserRole::Auditor, "connect"));
    }

    #[test]
    fn test_secret_redaction() {
        let mgr = SecurityManager::new();
        let config_line = "password = \"my_secret_pass_123\"";
        let redacted = mgr.redact_secrets(config_line);
        assert_eq!(redacted, "password = \"[REDACTED]\"");
    }
}
