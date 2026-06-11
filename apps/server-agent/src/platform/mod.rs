//! Platform abstraction layer for the server agent.
//!
//! Each supported OS provides the same set of operations behind a common
//! interface so the rest of the agent (control channel, connection handler)
//! stays platform-agnostic.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod generic;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use generic::*;

/// Where the local control channel listens. On Linux this is a Unix domain
/// socket path; on Windows (and other platforms) it is a localhost TCP address.
#[allow(dead_code)]
pub enum ControlEndpoint {
    /// Unix domain socket at the given filesystem path.
    UnixSocket(String),
    /// Localhost TCP socket at the given `host:port` address.
    LocalTcp(String),
}

/// Default config directory for the agent on this platform.
#[allow(dead_code)]
pub fn default_config_path() -> String {
    #[cfg(target_os = "linux")]
    {
        "/etc/ttgtiso-desk/agent.toml".to_string()
    }
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
        format!("{}\\TTGTiSO-Desk\\agent.toml", base)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        "agent.toml".to_string()
    }
}

/// Default audit log file path for the agent on this platform.
pub fn default_audit_log_path() -> std::path::PathBuf {
    #[cfg(target_os = "linux")]
    {
        std::path::PathBuf::from("/var/log/ttgtiso-desk/audit.log")
    }
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
        std::path::PathBuf::from(format!("{}\\TTGTiSO-Desk\\logs\\audit.log", base))
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        std::path::PathBuf::from("audit.log")
    }
}

/// Local control endpoint for this platform.
pub fn control_endpoint() -> ControlEndpoint {
    #[cfg(target_os = "linux")]
    {
        ControlEndpoint::UnixSocket("/var/lib/ttgtiso-desk/agent.sock".to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        // 127.0.0.1:2223 — local-only control channel for non-Unix platforms.
        ControlEndpoint::LocalTcp("127.0.0.1:2223".to_string())
    }
}

/// The agent host name reported in discovery beacons.
pub fn agent_hostname() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/hostname")
            .unwrap_or_else(|_| "localhost".to_string())
            .trim()
            .to_string()
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "localhost".to_string())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        "localhost".to_string()
    }
}
