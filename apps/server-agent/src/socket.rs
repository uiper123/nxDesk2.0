use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::broadcast;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn, error};
use session_manager::{LocalSessionManager, SessionManager};

/// Collect real system metrics from /proc on Linux
fn collect_system_metrics() -> serde_json::Value {
    let uptime = read_uptime_seconds();
    let (mem_total, mem_available) = read_memory_info();
    let load_avg = read_load_average();
    let hostname = read_hostname();
    let cpu_count = num_cpus();

    serde_json::json!({
        "hostname": hostname,
        "uptime_seconds": uptime,
        "cpu_count": cpu_count,
        "load_average_1m": load_avg.0,
        "load_average_5m": load_avg.1,
        "load_average_15m": load_avg.2,
        "memory_total_mb": mem_total / 1024,
        "memory_available_mb": mem_available / 1024,
        "memory_used_mb": (mem_total.saturating_sub(mem_available)) / 1024,
        "memory_usage_percent": if mem_total > 0 {
            ((mem_total.saturating_sub(mem_available)) as f64 / mem_total as f64 * 100.0) as u64
        } else { 0 },
    })
}

fn read_uptime_seconds() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(|v| v.to_string()))
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v as u64)
        .unwrap_or(0)
}

fn read_memory_info() -> (u64, u64) {
    let contents = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total_kb: u64 = 0;
    let mut available_kb: u64 = 0;

    for line in contents.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = parse_meminfo_value(line);
        } else if line.starts_with("MemAvailable:") {
            available_kb = parse_meminfo_value(line);
        }
    }
    (total_kb, available_kb)
}

fn parse_meminfo_value(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
}

fn read_load_average() -> (f64, f64, f64) {
    let contents = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let parts: Vec<&str> = contents.split_whitespace().collect();
    let l1 = parts.first().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let l5 = parts.get(1).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let l15 = parts.get(2).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    (l1, l5, l15)
}

fn read_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string()
}

fn num_cpus() -> usize {
    std::fs::read_to_string("/proc/cpuinfo")
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("processor"))
        .count()
        .max(1)
}

pub async fn run_uds_listener(
    path: String,
    session_mgr: Arc<LocalSessionManager>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    // 1. Cleanup old socket file if exists
    if std::fs::metadata(&path).is_ok() {
        let _ = std::fs::remove_file(&path);
    }

    // Bind Unix listener
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

    // Track server start time for uptime calculations
    let server_start = std::time::Instant::now();

    loop {
        tokio::select! {
            accept_res = listener.accept() => {
                match accept_res {
                    Ok((mut stream, _)) => {
                        let session_mgr_clone = session_mgr.clone();
                        let server_start = server_start;
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
                    Err(e) => {
                        warn!("Unix socket accept error: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Stopping Unix socket listener...");
                break;
            }
        }
    }

    // Clean up socket file on exit
    let _ = std::fs::remove_file(&path);
}

fn handle_command(
    command: &str,
    session_mgr: &Arc<LocalSessionManager>,
    server_start: std::time::Instant,
) -> String {
    match command {
        "status" => {
            let active_sessions = session_mgr.list_active_sessions()
                .map(|s| s.len())
                .unwrap_or(0);
            let system = collect_system_metrics();
            let agent_uptime = server_start.elapsed().as_secs();

            let response = serde_json::json!({
                "status": "OK",
                "agent_uptime_seconds": agent_uptime,
                "active_sessions": active_sessions,
                "system": system,
            });
            format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
        }
        "sessions" => {
            let sessions = session_mgr.list_active_sessions().unwrap_or_default();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let session_list: Vec<serde_json::Value> = sessions.iter().map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "username": s.username,
                    "display_id": s.display_id,
                    "start_time": s.start_time,
                    "duration_seconds": now.saturating_sub(s.start_time),
                })
            }).collect();

            let response = serde_json::json!({
                "sessions": session_list,
                "count": session_list.len(),
            });
            format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
        }
        "metrics" => {
            let system = collect_system_metrics();
            format!("{}\n", serde_json::to_string_pretty(&system).unwrap_or_default())
        }
        "applications" => {
            let apps = list_installed_applications();
            format!("{}\n", serde_json::to_string_pretty(&apps).unwrap_or_default())
        }
        "health" => {
            let response = serde_json::json!({
                "healthy": true,
                "agent_uptime_seconds": server_start.elapsed().as_secs(),
                "pid": std::process::id(),
            });
            format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
        }
        "users" => {
            let mut users = Vec::new();
            if let Ok(content) = std::fs::read_to_string("/etc/passwd") {
                for line in content.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 7 {
                        let username = parts[0];
                        let uid_str = parts[2];
                        let shell = parts[6];
                        if let Ok(uid) = uid_str.parse::<u32>() {
                            if uid >= 1000 && uid < 60000 && !shell.ends_with("nologin") && !shell.ends_with("false") {
                                users.push(username.to_string());
                            }
                        }
                    }
                }
            }
            let response = serde_json::json!({
                "users": users
            });
            format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
        }
        cmd if cmd.starts_with("launch") => {
            // Expected format: "launch <display_id> <command...>"
            let parts: Vec<&str> = command.splitn(3, ' ').collect();
            if parts.len() < 3 {
                let response = serde_json::json!({
                    "error": "Usage: launch <display_id> <command>"
                });
                return format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default());
            }
            
            let display_id = parts[1];
            let exec_cmd = parts[2];
            let display_str = format!(":{}", display_id);
            
            let child = std::process::Command::new("sh")
                .arg("-c")
                .arg(exec_cmd)
                .env("DISPLAY", &display_str)
                .env_remove("WAYLAND_DISPLAY")
                .env_remove("XDG_SESSION_TYPE")
                .env_remove("GDK_BACKEND")
                .env_remove("QT_QPA_PLATFORM")
                .spawn();
                
            match child {
                Ok(_) => {
                    let response = serde_json::json!({
                        "success": true,
                        "message": format!("Launched '{}' on display {}", exec_cmd, display_str)
                    });
                    format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                }
                Err(e) => {
                    let response = serde_json::json!({
                        "error": format!("Failed to spawn command: {}", e)
                    });
                    format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                }
            }
        }
        cmd if cmd.starts_with("start_session") => {
            // Ожидается формат: "start_session <username>"
            let parts: Vec<&str> = command.splitn(2, ' ').collect();
            if parts.len() < 2 || parts[1].trim().is_empty() {
                let response = serde_json::json!({
                    "error": "Usage: start_session <username>"
                });
                return format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default());
            }
            let username = parts[1].trim();
            match session_mgr.start_session(username) {
                Ok(info) => {
                    let response = serde_json::json!({
                        "success": true,
                        "session": {
                            "id": info.id,
                            "username": info.username,
                            "display_id": info.display_id,
                            "start_time": info.start_time,
                        }
                    });
                    format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                }
                Err(e) => {
                    let response = serde_json::json!({
                        "error": format!("Failed to start session: {}", e)
                    });
                    format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                }
            }
        }
        cmd if cmd.starts_with("stop_session") => {
            // Ожидается формат: "stop_session <session_id>"
            let parts: Vec<&str> = command.splitn(2, ' ').collect();
            if parts.len() < 2 || parts[1].trim().is_empty() {
                let response = serde_json::json!({
                    "error": "Usage: stop_session <session_id>"
                });
                return format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default());
            }
            let session_id = parts[1].trim();
            match session_mgr.stop_session(session_id) {
                Ok(_) => {
                    let response = serde_json::json!({
                        "success": true,
                        "message": format!("Session {} stopped", session_id)
                    });
                    format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                }
                Err(e) => {
                    let response = serde_json::json!({
                        "error": format!("Failed to stop session: {}", e)
                    });
                    format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                }
            }
        }
        cmd if cmd.starts_with("ensure_vnc") => {
            // Expected format: "ensure_vnc <display_id>"
            let parts: Vec<&str> = command.splitn(2, ' ').collect();
            if parts.len() < 2 {
                let response = serde_json::json!({
                    "error": "Usage: ensure_vnc <display_id>"
                });
                return format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default());
            }
            let display_str = parts[1].trim();
            if let Ok(display_id) = display_str.parse::<u32>() {
                let port = 5900 + display_id;
                // Check if VNC server is already active and listening on the port
                let is_active = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok();
                if is_active {
                    info!("VNC server already active on port {}", port);
                    let response = serde_json::json!({
                        "success": true,
                        "port": port
                    });
                    format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                } else {
                    let display_arg = format!(":{}", display_id);
                    
                    // Detect if it's Wayland or X11 and find the owner
                    let mut is_wayland = false;
                    let mut session_uid = 0;
                    let mut xdg_runtime = String::new();
                    
                    if display_id == 0 {
                        // Check for Wayland socket
                        if let Ok(entries) = std::fs::read_dir("/run/user") {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.is_dir() && path.join("wayland-0").exists() {
                                    is_wayland = true;
                                    xdg_runtime = path.to_string_lossy().to_string();
                                    if let Ok(meta) = std::fs::metadata(&path) {
                                        use std::os::unix::fs::MetadataExt;
                                        session_uid = meta.uid();
                                    }
                                    break;
                                }
                            }
                        }
                    }

                    // If not Wayland, check X11 socket owner
                    if !is_wayland {
                        let x_socket = format!("/tmp/.X11-unix/X{}", display_id);
                        if let Ok(meta) = std::fs::metadata(&x_socket) {
                            use std::os::unix::fs::MetadataExt;
                            session_uid = meta.uid();
                            xdg_runtime = format!("/run/user/{}", session_uid);
                        }
                    }

                    // Get username from uid
                    let username = if session_uid > 0 {
                        std::process::Command::new("id")
                            .arg("-nu")
                            .arg(session_uid.to_string())
                            .output()
                            .ok()
                            .and_then(|out| {
                                if out.status.success() {
                                    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| session_uid.to_string())
                    } else {
                        "root".to_string()
                    };

                    let mut cmd = if is_wayland {
                        info!("Wayland session detected for user {}. Using wayvnc on port {}", username, port);
                        let mut c = std::process::Command::new("runuser");
                        c.arg("-u").arg(&username).arg("--").arg("wayvnc");
                        c.arg("127.0.0.1").arg(port.to_string());
                        c.env("XDG_RUNTIME_DIR", &xdg_runtime);
                        c.env("WAYLAND_DISPLAY", "wayland-0");
                        c
                    } else {
                        info!("Starting x11vnc for user {} on display {} port {}", username, display_arg, port);
                        let x11vnc_path = if std::path::Path::new("/mnt/storage/projects/Nxdesc/local_bin/usr/bin/x11vnc").exists() {
                            "/mnt/storage/projects/Nxdesc/local_bin/usr/bin/x11vnc"
                        } else {
                            "x11vnc"
                        };
                        
                        let mut c = std::process::Command::new("runuser");
                        c.arg("-u").arg(&username).arg("--").arg(x11vnc_path);
                        
                        let port_str = port.to_string();
                        let mut args = vec![
                            "-display", &display_arg,
                            "-shared",
                            "-forever",
                            "-nopw",
                            "-rfbport", &port_str,
                            "-xkb",
                            "-localhost",
                            "-noipv6",
                        ];

                        if display_id == 0 {
                            args.push("-auth");
                            args.push("guess");
                        }

                        c.args(&args);
                        
                        if x11vnc_path != "x11vnc" {
                            c.env("LD_LIBRARY_PATH", "/mnt/storage/projects/Nxdesc/local_bin/usr/lib");
                        }
                        
                        c.env_remove("WAYLAND_DISPLAY");
                        c.env_remove("GDK_BACKEND");
                        c.env_remove("XDG_SESSION_TYPE");
                        c
                    };

                    match cmd.spawn() {
                        Ok(mut child) => {
                            let mut vnc_started = false;
                            for _ in 0..15 {
                                std::thread::sleep(std::time::Duration::from_millis(200));
                                if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                                    vnc_started = true;
                                    break;
                                }
                            }
                            
                            if vnc_started {
                                std::thread::spawn(move || { let _ = child.wait(); });
                                let response = serde_json::json!({
                                    "success": true,
                                    "port": port
                                });
                                format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                            } else {
                                let _ = child.kill();
                                let _ = child.wait();
                                let response = serde_json::json!({
                                    "error": format!("VNC server failed to start on display :{}. The display may not be accessible, or missing Wayland VNC server (wayvnc).", display_id)
                                });
                                format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                            }
                        }
                        Err(e) => {
                            let response = serde_json::json!({
                                "error": format!("Failed to spawn x11vnc: {}", e)
                            });
                            format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
                        }
                    }
                }
            } else {
                let response = serde_json::json!({
                    "error": "Invalid display ID"
                });
                format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
            }
        }
        _ => {
            let response = serde_json::json!({
                "error": "Unknown command",
                "available_commands": ["status", "sessions", "metrics", "health", "users", "start_session <username>", "stop_session <id>", "applications", "launch <display_id> <command>", "ensure_vnc <display_id>"]
            });
            format!("{}\n", serde_json::to_string_pretty(&response).unwrap_or_default())
        }
    }
}

fn list_installed_applications() -> serde_json::Value {
    let mut apps = Vec::new();
    let dirs = vec!["/usr/share/applications", "/usr/local/share/applications"];
    
    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let mut name = None;
                        let mut exec = None;
                        let mut no_display = false;
                        
                        for line in content.lines() {
                            if line.starts_with("Name=") && name.is_none() {
                                name = Some(line["Name=".len()..].trim().to_string());
                            } else if line.starts_with("Exec=") && exec.is_none() {
                                let mut raw_exec = line["Exec=".len()..].trim().to_string();
                                if let Some(idx) = raw_exec.find('%') {
                                    raw_exec.truncate(idx);
                                }
                                exec = Some(raw_exec.trim().to_string());
                            } else if line.starts_with("NoDisplay=") {
                                if line["NoDisplay=".len()..].trim().to_lowercase() == "true" {
                                    no_display = true;
                                }
                            }
                        }
                        
                        if let (Some(n), Some(e)) = (name, exec) {
                            if !no_display && !e.is_empty() {
                                apps.push(serde_json::json!({
                                    "name": n,
                                    "exec": e
                                }));
                            }
                        }
                    }
                }
            }
        }
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

    serde_json::json!({
        "applications": apps,
        "count": apps.len()
    })
}
