use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use shared_types::SessionKind;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphicalSession {
    pub username: String,
    pub uid: u32,
    pub home: String,
    pub display_id: u8,
    pub display: String,
    pub xauthority: Option<String>,
    pub wayland_display: Option<String>,
    pub session_kind: SessionKind,
    pub active_console: bool,
}

impl GraphicalSession {
    pub fn is_wayland(&self) -> bool {
        matches!(self.session_kind, SessionKind::Wayland)
    }

    pub fn is_x11(&self) -> bool {
        matches!(self.session_kind, SessionKind::X11 | SessionKind::Virtual)
    }
}

pub fn get_config_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "ttgtiso", "desk")
        .context("Failed to get system directory paths")?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn get_data_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "ttgtiso", "desk")
        .context("Failed to get system directory paths")?;
    Ok(dirs.data_dir().to_path_buf())
}

pub fn get_runtime_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "ttgtiso", "desk")
        .context("Failed to get system directory paths")?;
    Ok(dirs
        .runtime_dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| dirs.data_dir().to_path_buf()))
}

pub fn get_log_dir() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from("/var/log/ttgtiso-desk"))
    }
    #[cfg(target_os = "windows")]
    {
        let dirs = directories::ProjectDirs::from("com", "ttgtiso", "desk")
            .context("Failed to get system directory paths")?;
        Ok(dirs.data_local_dir().join("logs"))
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let dirs = directories::ProjectDirs::from("com", "ttgtiso", "desk")
            .context("Failed to get system directory paths")?;
        Ok(dirs.cache_dir().to_path_buf())
    }
}

pub fn execute_command(program: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new(program).args(args).output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn detect_graphical_session() -> Option<GraphicalSession> {
    #[cfg(target_os = "linux")]
    {
        detect_linux_graphical_session()
    }
    #[cfg(target_os = "windows")]
    {
        detect_windows_graphical_session()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

pub fn detect_preferred_graphical_session(username: Option<&str>) -> Option<GraphicalSession> {
    let sessions = list_graphical_sessions();
    if let Some(name) = username {
        if let Some(found) = sessions.iter().find(|s| s.username == name).cloned() {
            return Some(found);
        }
    }
    sessions.into_iter().next()
}

pub fn list_graphical_sessions() -> Vec<GraphicalSession> {
    #[cfg(target_os = "linux")]
    {
        list_linux_graphical_sessions()
    }
    #[cfg(target_os = "windows")]
    {
        detect_windows_graphical_session().into_iter().collect()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

pub fn should_attach_to_existing_session(mode: &config::ConnectionMode, session_type: &config::DesktopSessionType) -> bool {
    match (mode, session_type) {
        (config::ConnectionMode::Desktop, config::DesktopSessionType::Attach) => true,
        (config::ConnectionMode::Desktop, config::DesktopSessionType::Auto) => detect_graphical_session().is_some(),
        _ => false,
    }
}

pub fn desktop_launch_candidates(mode: &config::ConnectionMode, session_type: &config::DesktopSessionType) -> Vec<String> {
    if matches!(mode, config::ConnectionMode::App) {
        return vec!["xterm".to_string()];
    }

    match session_type {
        config::DesktopSessionType::Attach => Vec::new(),
        config::DesktopSessionType::Virtual | config::DesktopSessionType::Auto => vec![
            "fly-wm".to_string(),
            "startplasma-x11".to_string(),
            "gnome-session".to_string(),
            "xfce4-session".to_string(),
            "mate-session".to_string(),
            "cinnamon-session".to_string(),
            "/etc/X11/Xsession".to_string(),
            "~/.xinitrc".to_string(),
            "openbox".to_string(),
        ],
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_graphical_session() -> Option<GraphicalSession> {
    list_linux_graphical_sessions().into_iter().next()
}

#[cfg(target_os = "linux")]
fn list_linux_graphical_sessions() -> Vec<GraphicalSession> {
    use std::fs;
    use std::os::unix::fs::MetadataExt;

    let mut sessions = Vec::new();

    let resolve_username = |uid: u32| -> Option<(String, String)> {
        let passwd = fs::read_to_string("/etc/passwd").ok()?;
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
    };

    let detect_xauthority = |home: &str, runtime: &str, username: &str, uid: u32| -> Option<String> {
        let candidates = [
            format!("{home}/.Xauthority"),
            format!("{runtime}/Xauthority"),
            format!("{runtime}/gdm/Xauthority"),
            format!("/var/run/lightdm/{username}/xauthority"),
        ];
        for candidate in candidates {
            if PathBuf::from(&candidate).exists() {
                return Some(candidate);
            }
        }

        if let Ok(entries) = fs::read_dir(runtime) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("xauth") {
                        return Some(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }

        if let Ok(proc_dir) = fs::read_dir("/proc") {
            for entry in proc_dir.flatten() {
                let pid_path = entry.path();
                let comm = fs::read_to_string(pid_path.join("comm")).unwrap_or_default();
                let comm = comm.trim();
                if comm == "Xorg" || comm == "X" || comm == "Xwayland" {
                    let cmdline = fs::read_to_string(pid_path.join("cmdline")).unwrap_or_default();
                    let args: Vec<&str> = cmdline.split('\0').collect();
                    for (idx, arg) in args.iter().enumerate() {
                        if *arg == "-auth" {
                            if let Some(auth_path) = args.get(idx + 1) {
                                if PathBuf::from(auth_path).exists() {
                                    return Some((*auth_path).to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        let _ = uid;
        None
    };

    if let Ok(entries) = fs::read_dir("/tmp/.X11-unix") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(display) = name.strip_prefix('X') {
                    if let Ok(display_id) = display.parse::<u8>() {
                        let uid = entry.metadata().map(|m| m.uid()).unwrap_or(0);
                        let (username, home) = resolve_username(uid)
                            .unwrap_or_else(|| (format!("user_{uid}"), format!("/home/user_{uid}")));
                        let runtime = format!("/run/user/{uid}");
                        let xauthority = detect_xauthority(&home, &runtime, &username, uid);
                        let wayland_display = find_wayland_display(&runtime, display_id);

                        sessions.push(GraphicalSession {
                            username,
                            uid,
                            home,
                            display_id,
                            display: format!(":{display_id}"),
                            xauthority,
                            wayland_display,
                            session_kind: SessionKind::X11,
                            active_console: uid == 0,
                        });
                    }
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir("/run/user") {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Ok(uid) = entry.file_name().to_string_lossy().parse::<u32>() else {
                continue;
            };
            if let Some((username, home)) = resolve_username(uid) {
                if let Some(display_id) = find_wayland_display(&path.to_string_lossy(), 0)
                    .and_then(|w| w.strip_prefix("wayland-").and_then(|s| s.parse::<u8>().ok()))
                {
                    let xauthority = detect_xauthority(&home, &path.to_string_lossy(), &username, uid);
                    sessions.push(GraphicalSession {
                        username,
                        uid,
                        home,
                        display_id,
                        display: format!(":{display_id}"),
                        xauthority,
                        wayland_display: Some(format!("wayland-{display_id}")),
                        session_kind: SessionKind::Wayland,
                        active_console: uid == 0,
                    });
                }
            }
        }
    }

    sessions
}

#[cfg(target_os = "linux")]
fn find_wayland_display(runtime_dir: &str, display_id: u8) -> Option<String> {
    let runtime = PathBuf::from(runtime_dir);
    let candidates = [
        format!("wayland-{display_id}"),
        "wayland-0".to_string(),
        "wayland-1".to_string(),
    ];
    for candidate in candidates {
        if runtime.join(&candidate).exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn detect_windows_graphical_session() -> Option<GraphicalSession> {
    use windows::Win32::System::RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQuerySessionInformationW, WTS_SESSION_INFOW};
    let console_id = unsafe { WTSGetActiveConsoleSessionId() };
    if console_id == u32::MAX {
        return None;
    }
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string());
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    Some(GraphicalSession {
        username,
        uid: console_id,
        home,
        display_id: 0,
        display: "console".to_string(),
        xauthority: None,
        wayland_display: None,
        session_kind: SessionKind::Unknown,
        active_console: true,
    })
}

#[cfg(not(target_os = "windows"))]
fn detect_windows_graphical_session() -> Option<GraphicalSession> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_candidates_include_full_desktop_shells() {
        let candidates = desktop_launch_candidates(&config::ConnectionMode::Desktop, &config::DesktopSessionType::Auto);
        assert!(candidates.iter().any(|c| c == "gnome-session"));
        assert!(candidates.iter().any(|c| c == "startplasma-x11"));
    }
}
