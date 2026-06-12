use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

pub mod access;
pub use access::{AccessMode, AccessOutcome, AccessPolicy, HostContext};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum UserRole {
    User,
    Admin,
    SupportOperator,
    Auditor,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserCredentials {
    pub username: String,
    pub role: UserRole,
    pub password_hash: String,
    pub salt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CredentialsDatabase {
    pub users: Vec<UserCredentials>,
}

pub struct SecurityManager {
    db: CredentialsDatabase,
}

fn hash_password(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn generate_salt() -> String {
    #[cfg(unix)]
    {
        if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
            use std::io::Read;
            let mut buf = [0u8; 16];
            if file.read_exact(&mut buf).is_ok() {
                return buf.iter().map(|b| format!("{:02x}", b)).collect();
            }
        }
    }
    // Fallback or non-unix
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

impl SecurityManager {
    pub fn new() -> Self {
        let path = Path::new("users.toml");
        let db = if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => match toml::from_str::<CredentialsDatabase>(&content) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to parse users.toml: {}. Using defaults.",
                            e
                        );
                        Self::default_database()
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read users.toml: {}. Using defaults.", e);
                    Self::default_database()
                }
            }
        } else {
            let default_db = Self::default_database();
            if let Ok(toml_str) = toml::to_string_pretty(&default_db) {
                if let Err(e) = std::fs::write(path, toml_str) {
                    eprintln!("Warning: Failed to write default users.toml: {}", e);
                } else {
                    println!("Notice: Created default users.toml file. Adjust passwords in it for production.");
                }
            }
            default_db
        };

        Self { db }
    }

    fn default_database() -> CredentialsDatabase {
        let admin_salt = generate_salt();
        let operator_salt = generate_salt();
        let auditor_salt = generate_salt();

        CredentialsDatabase {
            users: vec![
                UserCredentials {
                    username: "admin".to_string(),
                    role: UserRole::Admin,
                    password_hash: hash_password("admin123", &admin_salt),
                    salt: admin_salt,
                },
                UserCredentials {
                    username: "operator".to_string(),
                    role: UserRole::SupportOperator,
                    password_hash: hash_password("operator123", &operator_salt),
                    salt: operator_salt,
                },
                UserCredentials {
                    username: "auditor".to_string(),
                    role: UserRole::Auditor,
                    password_hash: hash_password("auditor123", &auditor_salt),
                    salt: auditor_salt,
                },
            ],
        }
    }

    pub fn authenticate(&self, username: &str, secret: &str) -> Result<UserRole> {
        if secret.is_empty() {
            bail!("Authentication failed: Empty password");
        }

        // 1. Try local user database first (e.g. admin/admin123, operator/operator123)
        if let Some(user) = self.db.users.iter().find(|u| u.username == username) {
            let hash = hash_password(secret, &user.salt);
            if hash == user.password_hash {
                return Ok(user.role.clone());
            }
        }

        // 2. Fallback to system password verification (e.g. sudo password on Linux / Windows logon)
        if verify_system_password(username, secret) {
            return Ok(UserRole::Admin);
        }

        bail!("Authentication failed: Invalid credentials")
    }
}

#[cfg(unix)]
fn verify_system_password(username: &str, secret: &str) -> bool {
    let current_user = std::env::var("USER").unwrap_or_default();
    if username != "admin" && username != "root" && username != current_user {
        return false;
    }

    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = match Command::new("sudo")
        .args(["-S", "-p", "", "true"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = writeln!(stdin, "{}", secret);
    }

    child.wait().map(|status| status.success()).unwrap_or(false)
}

#[cfg(windows)]
fn verify_system_password(username: &str, secret: &str) -> bool {
    let current_user = std::env::var("USERNAME").unwrap_or_default();
    let target_user = if username == "admin" || username == "root" {
        &current_user
    } else {
        username
    };

    use windows::core::PCWSTR;
    use windows::Win32::Security::{
        LogonUserW, LOGON32_LOGON_INTERACTIVE, LOGON32_PROVIDER_DEFAULT,
    };

    let user_u16: Vec<u16> = target_user
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let pass_u16: Vec<u16> = secret.encode_utf16().chain(std::iter::once(0)).collect();
    let domain_u16: Vec<u16> = vec![0]; // Local computer

    unsafe {
        let mut token = windows::Win32::Foundation::HANDLE::default();
        let res = LogonUserW(
            PCWSTR(user_u16.as_ptr()),
            PCWSTR(domain_u16.as_ptr()),
            PCWSTR(pass_u16.as_ptr()),
            LOGON32_LOGON_INTERACTIVE,
            LOGON32_PROVIDER_DEFAULT,
            &mut token,
        );
        if res.is_ok() {
            let _ = windows::Win32::Foundation::CloseHandle(token);
            true
        } else {
            false
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn verify_system_password(_username: &str, _secret: &str) -> bool {
    false
}

impl SecurityManager {
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
