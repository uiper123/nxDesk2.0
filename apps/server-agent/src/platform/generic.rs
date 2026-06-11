//! Generic fallback platform layer for OSes without a dedicated backend.

use serde_json::{json, Value};
use tracing::info;

pub fn system_metrics() -> Value {
    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    json!({
        "hostname": "unknown",
        "os": std::env::consts::OS,
        "uptime_seconds": 0,
        "cpu_count": cpu_count,
        "load_average_1m": 0.0,
        "load_average_5m": 0.0,
        "load_average_15m": 0.0,
        "memory_total_mb": 0,
        "memory_available_mb": 0,
        "memory_used_mb": 0,
        "memory_usage_percent": 0,
    })
}

pub fn list_users() -> Value {
    let mut users = Vec::new();
    if let Ok(user) = std::env::var("USER") {
        if !user.is_empty() {
            users.push(user);
        }
    }
    json!({ "users": users })
}

pub fn list_applications() -> Value {
    json!({ "applications": [], "count": 0 })
}

pub fn launch_application(_display_id: u32, exec_cmd: &str) -> Value {
    info!("Launching '{}' (generic platform)", exec_cmd);
    match std::process::Command::new("sh").arg("-c").arg(exec_cmd).spawn() {
        Ok(_) => json!({ "success": true, "message": format!("Launched '{}'", exec_cmd) }),
        Err(e) => json!({ "error": format!("Failed to spawn command: {}", e) }),
    }
}

pub fn ensure_vnc(_display_id: u32) -> Value {
    json!({ "error": "VNC is not supported on this platform" })
}
