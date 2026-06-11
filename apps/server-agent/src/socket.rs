//! Cross-platform local control channel for the server agent.
//!
//! On Linux the control channel is a Unix domain socket (preserving the
//! existing API-server contract). On Windows and other platforms it is a
//! localhost-only TCP socket. Both speak the same line-oriented text protocol
//! and return identical JSON responses, so the rest of the system is unchanged.

use crate::platform::{self, ControlEndpoint};
use session_manager::{LocalSessionManager, SessionManager};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// Start the control listener on this platform's control endpoint.
pub async fn run_control_listener(
    session_mgr: Arc<LocalSessionManager>,
    shutdown_rx: broadcast::Receiver<()>,
) {
    match platform::control_endpoint() {
        ControlEndpoint::UnixSocket(path) => {
            #[cfg(unix)]
            {
                run_uds_listener(path, session_mgr, shutdown_rx).await;
            }
            #[cfg(not(unix))]
            {
                let _ = (path, session_mgr, shutdown_rx);
                error!("Unix socket control endpoint is not supported on this platform");
            }
        }
        ControlEndpoint::LocalTcp(addr) => {
            run_tcp_control_listener(addr, session_mgr, shutdown_rx).await;
        }
    }
}

/// Backwards-compatible entry point used by tests and existing callers: run the
/// Unix-domain-socket control listener at the given path.
#[cfg(unix)]
pub async fn run_uds_listener(
    path: String,
    session_mgr: Arc<LocalSessionManager>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    use tokio::net::UnixListener;

    if std::fs::metadata(&path).is_ok() {
        let _ = std::fs::remove_file(&path);
    }
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = match UnixListener::bind(&path) {
        Ok(l) => {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)) {
                warn!("Failed to set 666 permissions on agent.sock: {}", e);
            }
            info!("Local control Unix socket bound to: {}", path);
            l
        }
        Err(e) => {
            error!("Failed to bind local control Unix socket: {}", e);
            return;
        }
    };

    let server_start = std::time::Instant::now();

    loop {
        tokio::select! {
            accept_res = listener.accept() => {
                match accept_res {
                    Ok((mut stream, _)) => {
                        let session_mgr_clone = session_mgr.clone();
                        tokio::spawn(async move {
                            let mut buf = [0u8; 4096];
                            if let Ok(n) = stream.read(&mut buf).await {
                                let request = String::from_utf8_lossy(&buf[..n]);
                                let command = request.trim();
                                let response = handle_command(command, &session_mgr_clone, server_start);
                                let _ = stream.write_all(response.as_bytes()).await;
                            }
                        });
                    }
                    Err(e) => warn!("Unix socket accept error: {}", e),
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Stopping Unix socket listener...");
                break;
            }
        }
    }

    let _ = std::fs::remove_file(&path);
}

/// Localhost TCP control listener (Windows and other non-Unix platforms).
pub async fn run_tcp_control_listener(
    addr: String,
    session_mgr: Arc<LocalSessionManager>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    use tokio::net::TcpListener;

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            info!("Local control TCP socket bound to: {}", addr);
            l
        }
        Err(e) => {
            error!("Failed to bind local control TCP socket {}: {}", addr, e);
            return;
        }
    };

    let server_start = std::time::Instant::now();

    loop {
        tokio::select! {
            accept_res = listener.accept() => {
                match accept_res {
                    Ok((mut stream, _)) => {
                        let session_mgr_clone = session_mgr.clone();
                        tokio::spawn(async move {
                            let mut buf = [0u8; 4096];
                            if let Ok(n) = stream.read(&mut buf).await {
                                let request = String::from_utf8_lossy(&buf[..n]);
                                let command = request.trim();
                                let response = handle_command(command, &session_mgr_clone, server_start);
                                let _ = stream.write_all(response.as_bytes()).await;
                            }
                        });
                    }
                    Err(e) => warn!("TCP control accept error: {}", e),
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Stopping TCP control listener...");
                break;
            }
        }
    }
}

fn pretty(value: serde_json::Value) -> String {
    format!("{}\n", serde_json::to_string_pretty(&value).unwrap_or_default())
}

fn handle_command(
    command: &str,
    session_mgr: &Arc<LocalSessionManager>,
    server_start: std::time::Instant,
) -> String {
    match command {
        "status" => {
            let active_sessions = session_mgr
                .list_active_sessions()
                .map(|s| s.len())
                .unwrap_or(0);
            let system = platform::system_metrics();
            let agent_uptime = server_start.elapsed().as_secs();
            pretty(serde_json::json!({
                "status": "OK",
                "agent_uptime_seconds": agent_uptime,
                "active_sessions": active_sessions,
                "system": system,
            }))
        }
        "sessions" => {
            let sessions = session_mgr.list_active_sessions().unwrap_or_default();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let session_list: Vec<serde_json::Value> = sessions
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id,
                        "username": s.username,
                        "display_id": s.display_id,
                        "start_time": s.start_time,
                        "duration_seconds": now.saturating_sub(s.start_time),
                        "session_kind": s.session_kind,
                    })
                })
                .collect();
            pretty(serde_json::json!({
                "sessions": session_list,
                "count": session_list.len(),
            }))
        }
        "metrics" => pretty(platform::system_metrics()),
        "applications" => pretty(platform::list_applications()),
        "health" => pretty(serde_json::json!({
            "healthy": true,
            "agent_uptime_seconds": server_start.elapsed().as_secs(),
            "pid": std::process::id(),
        })),
        "users" => pretty(platform::list_users()),
        cmd if cmd.starts_with("launch") => {
            let parts: Vec<&str> = command.splitn(3, ' ').collect();
            if parts.len() < 3 {
                return pretty(serde_json::json!({
                    "error": "Usage: launch <display_id> <command>"
                }));
            }
            let Ok(display_id) = parts[1].parse::<u32>() else {
                return pretty(serde_json::json!({ "error": "Invalid display ID" }));
            };
            pretty(platform::launch_application(display_id, parts[2]))
        }
        cmd if cmd.starts_with("start_session") => {
            let parts: Vec<&str> = command.splitn(2, ' ').collect();
            if parts.len() < 2 || parts[1].trim().is_empty() {
                return pretty(serde_json::json!({ "error": "Usage: start_session <username>" }));
            }
            let username = parts[1].trim();
            match session_mgr.start_session(username) {
                Ok(info) => pretty(serde_json::json!({
                    "success": true,
                    "session": {
                        "id": info.id,
                        "username": info.username,
                        "display_id": info.display_id,
                        "start_time": info.start_time,
                        "session_kind": info.session_kind,
                    }
                })),
                Err(e) => pretty(serde_json::json!({
                    "error": format!("Failed to start session: {}", e)
                })),
            }
        }
        cmd if cmd.starts_with("stop_session") => {
            let parts: Vec<&str> = command.splitn(2, ' ').collect();
            if parts.len() < 2 || parts[1].trim().is_empty() {
                return pretty(serde_json::json!({ "error": "Usage: stop_session <session_id>" }));
            }
            let session_id = parts[1].trim();
            match session_mgr.stop_session(session_id) {
                Ok(_) => pretty(serde_json::json!({
                    "success": true,
                    "message": format!("Session {} stopped", session_id)
                })),
                Err(e) => pretty(serde_json::json!({
                    "error": format!("Failed to stop session: {}", e)
                })),
            }
        }
        cmd if cmd.starts_with("ensure_vnc") => {
            let parts: Vec<&str> = command.splitn(2, ' ').collect();
            if parts.len() < 2 {
                return pretty(serde_json::json!({ "error": "Usage: ensure_vnc <display_id>" }));
            }
            match parts[1].trim().parse::<u32>() {
                Ok(display_id) => pretty(platform::ensure_vnc(display_id)),
                Err(_) => pretty(serde_json::json!({ "error": "Invalid display ID" })),
            }
        }
        _ => pretty(serde_json::json!({
            "error": "Unknown command",
            "available_commands": ["status", "sessions", "metrics", "health", "users", "start_session <username>", "stop_session <id>", "applications", "launch <display_id> <command>", "ensure_vnc <display_id>"]
        })),
    }
}
