//! Windows implementation of the agent platform layer.
//!
//! Windows has a single interactive console session, so "sessions" and
//! "displays" map onto the active desktop. Screen capture and input injection
//! are handled in the `video-pipeline` and `input-injector` crates via the
//! Win32 GDI / SendInput APIs; this module supplies system metrics, the user
//! list, installed-application discovery, and process launching.

use serde_json::{json, Value};
use std::process::Command;
use tracing::info;

/// Collect system metrics using Win32 APIs.
pub fn system_metrics() -> Value {
    let hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string());
    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let (mem_total_mb, mem_avail_mb, mem_pct) = memory_info();
    let uptime = uptime_seconds();

    json!({
        "hostname": hostname,
        "os": "windows",
        "uptime_seconds": uptime,
        "cpu_count": cpu_count,
        // Windows has no load-average concept; report 0.0 for wire compatibility.
        "load_average_1m": 0.0,
        "load_average_5m": 0.0,
        "load_average_15m": 0.0,
        "memory_total_mb": mem_total_mb,
        "memory_available_mb": mem_avail_mb,
        "memory_used_mb": mem_total_mb.saturating_sub(mem_avail_mb),
        "memory_usage_percent": mem_pct,
    })
}

#[cfg(target_os = "windows")]
fn memory_info() -> (u64, u64, u64) {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    unsafe {
        let mut status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if GlobalMemoryStatusEx(&mut status).is_ok() {
            let total_mb = status.ullTotalPhys / (1024 * 1024);
            let avail_mb = status.ullAvailPhys / (1024 * 1024);
            let pct = status.dwMemoryLoad as u64;
            (total_mb, avail_mb, pct)
        } else {
            (0, 0, 0)
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn memory_info() -> (u64, u64, u64) {
    (0, 0, 0)
}

#[cfg(target_os = "windows")]
fn uptime_seconds() -> u64 {
    use windows::Win32::System::SystemInformation::GetTickCount64;
    unsafe { GetTickCount64() / 1000 }
}

#[cfg(not(target_os = "windows"))]
fn uptime_seconds() -> u64 {
    0
}

/// List interactive users. On Windows we report the current user; the
/// remote-desktop model targets the active console session.
pub fn list_users() -> Value {
    let mut users = Vec::new();
    if let Ok(user) = std::env::var("USERNAME") {
        if !user.is_empty() {
            users.push(user);
        }
    }
    json!({ "users": users })
}

/// Discover installed applications by scanning the Start Menu shortcut folders.
pub fn list_applications() -> Value {
    let mut apps: Vec<Value> = Vec::new();
    let mut dirs: Vec<String> = Vec::new();

    if let Ok(program_data) = std::env::var("ProgramData") {
        dirs.push(format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs",
            program_data
        ));
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs.push(format!("{}\\Microsoft\\Windows\\Start Menu\\Programs", appdata));
    }

    fn scan_dir(dir: &std::path::Path, apps: &mut Vec<Value>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, apps);
                } else if path
                    .extension()
                    .map(|e| e.eq_ignore_ascii_case("lnk"))
                    .unwrap_or(false)
                {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        apps.push(json!({
                            "name": stem,
                            "exec": path.to_string_lossy().to_string(),
                        }));
                    }
                }
            }
        }
    }

    for dir in &dirs {
        scan_dir(std::path::Path::new(dir), &mut apps);
    }

    apps.sort_by(|a, b| {
        let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });
    apps.dedup_by(|a, b| {
        let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        name_a == name_b
    });

    json!({ "applications": apps, "count": apps.len() })
}

/// Launch an application on the active desktop. `display_id` is ignored on
/// Windows (there is a single interactive session); `exec_cmd` is the program
/// (and arguments) or a `.lnk` shortcut path to start.
pub fn launch_application(_display_id: u32, exec_cmd: &str) -> Value {
    info!("Launching '{}' on the active Windows desktop", exec_cmd);

    // Use `cmd /C start` so both executables and shortcuts (.lnk) work, and the
    // process is detached from the agent.
    let spawn = Command::new("cmd")
        .args(["/C", "start", "", exec_cmd])
        .spawn();

    match spawn {
        Ok(_) => json!({
            "success": true,
            "message": format!("Launched '{}' on the active desktop", exec_cmd)
        }),
        Err(e) => json!({ "error": format!("Failed to spawn command: {}", e) }),
    }
}

/// On Windows, screen sharing is served by the agent's own capture/encode
/// pipeline rather than an external VNC server, so there is nothing to start.
/// The agent's TCP port is the single streaming endpoint.
pub fn ensure_vnc(_display_id: u32) -> Value {
    json!({
        "success": true,
        "message": "Windows agent streams directly over its TCP port; no external VNC server is required.",
        "native_streaming": true
    })
}
