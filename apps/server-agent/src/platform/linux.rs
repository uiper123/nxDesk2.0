//! Linux (X11 / Wayland on Astra) implementation of the agent platform layer.

use serde_json::{json, Value};
use std::process::Command;
use tracing::{info, warn};

/// Owner and environment details for a graphical display (X11 or Wayland).
struct DisplayOwner {
    username: String,
    uid: u32,
    home: String,
    xdg_runtime: String,
    wayland_socket: Option<String>,
}

fn lookup_passwd_entry(uid: u32) -> Option<(String, String)> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    for line in passwd.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 6 {
            if let Ok(parsed_uid) = parts[2].parse::<u32>() {
                if parsed_uid == uid {
                    return Some((parts[0].to_string(), parts[5].to_string()));
                }
            }
        }
    }
    None
}

fn find_wayland_socket(runtime_dir: &std::path::Path, display_id: u32) -> Option<String> {
    let candidates = [
        format!("wayland-{}", display_id),
        "wayland-0".to_string(),
        "wayland-1".to_string(),
    ];
    for candidate in candidates {
        let path = runtime_dir.join(&candidate);
        if path.exists() {
            return Some(candidate);
        }
    }
    None
}

fn resolve_display_owner(display_id: u32) -> Option<DisplayOwner> {
    use std::os::unix::fs::MetadataExt;

    let x_socket = format!("/tmp/.X11-unix/X{}", display_id);
    if let Ok(meta) = std::fs::metadata(&x_socket) {
        let uid = meta.uid();
        let (username, home) =
            lookup_passwd_entry(uid).unwrap_or_else(|| ("root".to_string(), "/root".to_string()));
        let xdg_runtime = format!("/run/user/{}", uid);
        let wayland_socket = find_wayland_socket(std::path::Path::new(&xdg_runtime), display_id);
        return Some(DisplayOwner {
            username,
            uid,
            home,
            xdg_runtime,
            wayland_socket,
        });
    }

    if let Ok(entries) = std::fs::read_dir("/run/user") {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Ok(uid) = entry.file_name().to_string_lossy().parse::<u32>() else {
                continue;
            };
            if let Some(socket) = find_wayland_socket(&path, display_id) {
                let (username, home) = lookup_passwd_entry(uid)
                    .unwrap_or_else(|| (format!("user_{}", uid), format!("/home/user_{}", uid)));
                return Some(DisplayOwner {
                    username,
                    uid,
                    home,
                    xdg_runtime: path.to_string_lossy().to_string(),
                    wayland_socket: Some(socket),
                });
            }
        }
    }

    None
}

fn find_xauthority(owner: &DisplayOwner) -> Option<String> {
    let candidates = [
        format!("{}/.Xauthority", owner.home),
        format!("{}/Xauthority", owner.xdg_runtime),
        format!("{}/gdm/Xauthority", owner.xdg_runtime),
        format!("/var/run/lightdm/{}/xauthority", owner.username),
    ];
    for candidate in candidates {
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }

    if let Ok(entries) = std::fs::read_dir(&owner.xdg_runtime) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("xauth") {
                    return Some(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir("/run/sddm") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("xauth") {
                    return Some(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }

    if let Some(xauth) = find_xauthority_from_xorg_proc(owner.uid) {
        return Some(xauth);
    }

    None
}

fn find_xauthority_from_xorg_proc(_owner_uid: u32) -> Option<String> {
    let proc_dir = std::fs::read_dir("/proc").ok()?;
    for entry in proc_dir.flatten() {
        let pid_path = entry.path();
        let comm_path = pid_path.join("comm");
        if let Ok(comm) = std::fs::read_to_string(&comm_path) {
            let comm = comm.trim();
            if comm == "Xorg" || comm == "X" || comm == "Xwayland" {
                let cmdline_path = pid_path.join("cmdline");
                if let Ok(cmdline) = std::fs::read_to_string(&cmdline_path) {
                    let args: Vec<&str> = cmdline.split('\0').collect();
                    for (i, arg) in args.iter().enumerate() {
                        if *arg == "-auth" {
                            if let Some(auth_path) = args.get(i + 1) {
                                if std::path::Path::new(auth_path).exists() {
                                    return Some(auth_path.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn build_user_launch_command(owner: &DisplayOwner, display_id: u32, exec_cmd: &str) -> Command {
    let display_str = format!(":{}", display_id);
    let mut cmd = Command::new("runuser");
    cmd.arg("-u")
        .arg(&owner.username)
        .arg("--")
        .arg("sh")
        .arg("-c")
        .arg(exec_cmd);
    cmd.env("HOME", &owner.home);
    cmd.env("USER", &owner.username);
    cmd.env("LOGNAME", &owner.username);
    cmd.env("XDG_RUNTIME_DIR", &owner.xdg_runtime);
    cmd.env(
        "DBUS_SESSION_BUS_ADDRESS",
        format!("unix:path={}/bus", owner.xdg_runtime),
    );
    cmd.env("DISPLAY", &display_str);

    if let Some(ref wayland_socket) = owner.wayland_socket {
        cmd.env("WAYLAND_DISPLAY", wayland_socket);
        cmd.env("XDG_SESSION_TYPE", "wayland");
    } else {
        cmd.env_remove("WAYLAND_DISPLAY");
        cmd.env("XDG_SESSION_TYPE", "x11");
        cmd.env_remove("GDK_BACKEND");
        cmd.env_remove("QT_QPA_PLATFORM");
        if let Some(xauth) = find_xauthority(owner) {
            cmd.env("XAUTHORITY", xauth);
        }
    }

    cmd
}

// --------------------------- public platform API ---------------------------

/// Collect real system metrics from /proc.
pub fn system_metrics() -> Value {
    let uptime = read_uptime_seconds();
    let (mem_total, mem_available) = read_memory_info();
    let load_avg = read_load_average();
    let hostname = read_hostname();
    let cpu_count = num_cpus();

    json!({
        "hostname": hostname,
        "os": "linux",
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
    let l1 = parts
        .first()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    let l5 = parts
        .get(1)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    let l15 = parts
        .get(2)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
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

/// List interactive local user accounts.
pub fn list_users() -> Value {
    let mut users = Vec::new();
    if let Ok(content) = std::fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 7 {
                let username = parts[0];
                let uid_str = parts[2];
                let shell = parts[6];
                if let Ok(uid) = uid_str.parse::<u32>() {
                    if (1000..60000).contains(&uid)
                        && !shell.ends_with("nologin")
                        && !shell.ends_with("false")
                    {
                        users.push(username.to_string());
                    }
                }
            }
        }
    }
    json!({ "users": users })
}

/// List installed desktop applications.
pub fn list_applications() -> Value {
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
                            } else if let Some(stripped) = line.strip_prefix("NoDisplay=") {
                                if stripped.trim().to_lowercase() == "true" {
                                    no_display = true;
                                }
                            }
                        }

                        if let (Some(n), Some(e)) = (name, exec) {
                            if !no_display && !e.is_empty() {
                                apps.push(json!({ "name": n, "exec": e }));
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

    json!({ "applications": apps, "count": apps.len() })
}

/// Launch `exec_cmd` on the graphical session identified by `display_id`.
pub fn launch_application(display_id: u32, exec_cmd: &str) -> Value {
    let Some(owner) = resolve_display_owner(display_id) else {
        return json!({
            "error": format!("Display :{} not found. No X11 socket or Wayland session detected.", display_id)
        });
    };

    info!(
        "Launching '{}' on display :{} as user {} (uid={}, wayland={})",
        exec_cmd,
        display_id,
        owner.username,
        owner.uid,
        owner.wayland_socket.is_some()
    );

    let mut launch_cmd = build_user_launch_command(&owner, display_id, exec_cmd);
    launch_cmd.stdout(std::process::Stdio::null());
    launch_cmd.stderr(std::process::Stdio::piped());

    match launch_cmd.spawn() {
        Ok(mut child) => {
            std::thread::sleep(std::time::Duration::from_millis(500));
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => {
                    let mut stderr_text = String::new();
                    if let Some(mut pipe) = child.stderr.take() {
                        use std::io::Read;
                        let _ = pipe.read_to_string(&mut stderr_text);
                    }
                    let detail = stderr_text.lines().last().unwrap_or("").to_string();
                    json!({
                        "error": format!(
                            "Command '{}' exited immediately with {}{}",
                            exec_cmd,
                            status,
                            if detail.is_empty() { String::new() } else { format!(": {}", detail) }
                        )
                    })
                }
                _ => {
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                    json!({
                        "success": true,
                        "message": format!("Launched '{}' on display :{} as {}", exec_cmd, display_id, owner.username)
                    })
                }
            }
        }
        Err(e) => json!({ "error": format!("Failed to spawn command: {}", e) }),
    }
}

/// Ensure a VNC server is running for the given display, starting one if needed.
pub fn ensure_vnc(display_id: u32) -> Value {
    let port = 5900 + display_id;
    let is_active = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok();
    if is_active {
        info!("VNC server already active on port {}", port);
        return json!({ "success": true, "port": port });
    }

    let Some(owner) = resolve_display_owner(display_id) else {
        return json!({
            "error": format!("Display :{} not found. No X11 socket or Wayland session detected.", display_id)
        });
    };

    let has_wayland = owner.wayland_socket.is_some();
    let has_x11 = std::path::Path::new(&format!("/tmp/.X11-unix/X{}", display_id)).exists();

    let mut cmd = if has_wayland {
        let wayland_socket = owner
            .wayland_socket
            .clone()
            .unwrap_or_else(|| "wayland-0".to_string());

        let wayvnc_available = Command::new("sh")
            .arg("-c")
            .arg("command -v wayvnc")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if wayvnc_available {
            info!(
                "Wayland session detected for user {} (socket {}). Using wayvnc on port {}",
                owner.username, wayland_socket, port
            );
            let mut c = Command::new("runuser");
            c.arg("-u").arg(&owner.username).arg("--").arg("wayvnc");
            c.arg(format!("127.0.0.1:{}", port));
            c.env("XDG_RUNTIME_DIR", &owner.xdg_runtime);
            c.env("WAYLAND_DISPLAY", &wayland_socket);
            c.env("HOME", &owner.home);
            c.env("USER", &owner.username);
            c
        } else if has_x11 {
            let display_arg = format!(":{}", display_id);
            info!(
                "Wayland session with Xwayland for user {} on display {}. wayvnc not installed, falling back to x11vnc on port {}",
                owner.username, display_arg, port
            );

            let x11vnc_available = Command::new("sh")
                .arg("-c")
                .arg("command -v x11vnc")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !x11vnc_available {
                return json!({
                    "error": format!(
                        "Wayland session on display :{} requires wayvnc (preferred) or x11vnc (fallback via Xwayland). Neither is installed.",
                        display_id
                    )
                });
            }

            let mut c = Command::new("runuser");
            c.arg("-u").arg(&owner.username).arg("--").arg("x11vnc");
            let port_str = port.to_string();
            c.args([
                "-display",
                &display_arg,
                "-shared",
                "-forever",
                "-nopw",
                "-rfbport",
                &port_str,
                "-xkb",
                "-localhost",
                "-noxdamage",
                "-noxrecord",
                "-noxfixes",
            ]);
            let xauth_path = find_xauthority(&owner);
            if let Some(ref xauth) = xauth_path {
                info!("Using XAUTHORITY for Xwayland: {}", xauth);
                c.args(["-auth", xauth]);
                c.env("XAUTHORITY", xauth);
            }
            c.env("DISPLAY", &display_arg);
            c.env("XDG_RUNTIME_DIR", &owner.xdg_runtime);
            c.env_remove("WAYLAND_DISPLAY");
            c.env_remove("GDK_BACKEND");
            c.env_remove("XDG_SESSION_TYPE");
            c
        } else {
            return json!({
                "error": format!(
                    "Wayland session on display :{} requires wayvnc, but it is not installed. Install it: 'pacman -S wayvnc' or 'apt install wayvnc'.",
                    display_id
                )
            });
        }
    } else {
        let display_arg = format!(":{}", display_id);
        info!(
            "Starting x11vnc for user {} on display {} port {}",
            owner.username, display_arg, port
        );

        let x11vnc_available = Command::new("sh")
            .arg("-c")
            .arg("command -v x11vnc")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !x11vnc_available {
            return json!({
                "error": format!(
                    "X11 session on display :{} requires x11vnc, but it is not installed. Install it with the system package manager (e.g. 'pacman -S x11vnc' or 'apt install x11vnc').",
                    display_id
                )
            });
        }

        let mut c = Command::new("runuser");
        c.arg("-u").arg(&owner.username).arg("--").arg("x11vnc");
        let port_str = port.to_string();
        c.args([
            "-display",
            &display_arg,
            "-shared",
            "-forever",
            "-nopw",
            "-rfbport",
            &port_str,
            "-xkb",
            "-localhost",
        ]);
        let xauth_path = find_xauthority(&owner);
        if let Some(ref xauth) = xauth_path {
            info!("Using XAUTHORITY: {}", xauth);
            c.args(["-auth", xauth]);
            c.env("XAUTHORITY", xauth);
        } else if display_id == 0 {
            info!("No explicit XAUTHORITY found for display :0, using -auth guess");
            c.args(["-auth", "guess"]);
        }
        c.env("DISPLAY", &display_arg);
        c.env_remove("WAYLAND_DISPLAY");
        c.env_remove("GDK_BACKEND");
        c.env_remove("XDG_SESSION_TYPE");
        c
    };

    cmd.stderr(std::process::Stdio::piped());
    match cmd.spawn() {
        Ok(mut child) => {
            let mut vnc_started = false;
            for _ in 0..15 {
                if let Ok(Some(_)) = child.try_wait() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
                if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                    vnc_started = true;
                    break;
                }
            }

            if vnc_started {
                info!("VNC server successfully started on port {}", port);
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                json!({ "success": true, "port": port })
            } else {
                let mut stderr_text = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    use std::io::Read;
                    let _ = pipe.read_to_string(&mut stderr_text);
                }
                let _ = child.kill();
                let _ = child.wait();
                let mut detail = stderr_text.lines().last().unwrap_or("").to_string();
                if stderr_text.contains("Virtual Pointer") || stderr_text.contains("Screencopy") {
                    detail = format!(
                        "{} (Compositor protocol mismatch: wayvnc is only compatible with wlroots-based compositors like Sway/Hyprland. GNOME and KDE Plasma Wayland are not supported. Please log out and switch your session to X11 at the login screen.)",
                        detail
                    );
                }
                let msg = if has_wayland {
                    format!(
                        "Wayland VNC server (wayvnc) failed to start on display :{}{}.",
                        display_id,
                        if detail.is_empty() {
                            String::new()
                        } else {
                            format!(": {}", detail)
                        }
                    )
                } else {
                    format!(
                        "x11vnc failed to start on display :{}. The display may not be accessible or XAUTHORITY is incorrect{}.",
                        display_id,
                        if detail.is_empty() { String::new() } else { format!(": {}", detail) }
                    )
                };
                warn!("{}", msg);
                json!({ "error": msg })
            }
        }
        Err(e) => {
            let binary_name = if has_wayland { "wayvnc" } else { "x11vnc" };
            json!({ "error": format!("Failed to spawn {}: {}", binary_name, e) })
        }
    }
}
