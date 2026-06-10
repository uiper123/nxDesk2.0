use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AgentConfig {
    pub bind_address: String,
    pub port: u16,
    pub session_limits: SessionLimits,
    pub security_policy: SecurityPolicy,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 22,
            session_limits: SessionLimits::default(),
            security_policy: SecurityPolicy::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SessionLimits {
    pub max_concurrent_sessions: u32,
    pub session_timeout_seconds: u32,
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            max_concurrent_sessions: 5,
            session_timeout_seconds: 3600,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SecurityPolicy {
    pub allow_password_auth: bool,
    pub enable_audit_logs: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allow_password_auth: true,
            enable_audit_logs: true,
        }
    }
}

/// Attempt to load configuration from known file paths in priority order.
/// Falls back to defaults if no config file is found.
pub fn load_config() -> Result<AgentConfig> {
    let search_paths = [
        "/etc/ttgtiso-desk/agent.toml",
        "/etc/ttgtiso-desk/agent.json",
        "agent.toml",
        "config/agent.toml",
    ];

    for path in &search_paths {
        let p = Path::new(path);
        if p.exists() {
            return load_config_from_file(p);
        }
    }

    tracing::info!("No config file found at known paths, using secure defaults");
    Ok(AgentConfig::default())
}

/// Load configuration from a specific file path.
/// Supports both TOML and JSON formats based on file extension.
pub fn load_config_from_file(path: &Path) -> Result<AgentConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let config: AgentConfig = match ext {
        "toml" => {
            toml::from_str(&contents)
                .with_context(|| format!("Failed to parse TOML config: {:?}", path))?
        }
        "json" => {
            serde_json::from_str(&contents)
                .with_context(|| format!("Failed to parse JSON config: {:?}", path))?
        }
        _ => {
            // Try TOML first, then JSON
            toml::from_str(&contents)
                .or_else(|_| serde_json::from_str(&contents).map_err(anyhow::Error::from))
                .with_context(|| format!("Failed to parse config file: {:?}", path))?
        }
    };

    tracing::info!("Loaded configuration from {:?}", path);
    Ok(config)
}

/// Save configuration to a TOML file
pub fn save_config(config: &AgentConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = toml::to_string_pretty(config)
        .context("Failed to serialize configuration to TOML")?;
    std::fs::write(path, contents)
        .with_context(|| format!("Failed to write config file: {:?}", path))?;

    tracing::info!("Configuration saved to {:?}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_loading() {
        let config = load_config().unwrap();
        assert_eq!(config.port, 22);
        assert!(config.security_policy.allow_password_auth);
    }

    #[test]
    fn test_config_deserialization() {
        let json_data = r#"{
            "bind_address": "0.0.0.0",
            "port": 2222,
            "session_limits": {
                "max_concurrent_sessions": 10,
                "session_timeout_seconds": 1800
            },
            "security_policy": {
                "allow_password_auth": false,
                "enable_audit_logs": true
            }
        }"#;
        
        let parsed: AgentConfig = serde_json::from_str(json_data).unwrap();
        assert_eq!(parsed.port, 2222);
        assert!(!parsed.security_policy.allow_password_auth);
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_data = r#"
bind_address = "0.0.0.0"
port = 2222

[session_limits]
max_concurrent_sessions = 10
session_timeout_seconds = 1800

[security_policy]
allow_password_auth = false
enable_audit_logs = true
"#;
        let parsed: AgentConfig = toml::from_str(toml_data).unwrap();
        assert_eq!(parsed.port, 2222);
        assert_eq!(parsed.session_limits.max_concurrent_sessions, 10);
        assert!(!parsed.security_policy.allow_password_auth);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = AgentConfig {
            bind_address: "192.168.1.100".to_string(),
            port: 8022,
            session_limits: SessionLimits {
                max_concurrent_sessions: 20,
                session_timeout_seconds: 7200,
            },
            security_policy: SecurityPolicy {
                allow_password_auth: false,
                enable_audit_logs: true,
            },
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AgentConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, parsed);
    }
}
