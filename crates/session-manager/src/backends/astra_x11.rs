use crate::traits::{SessionBackend, UserSession};
use anyhow::{Context, Result};
use shared_types::{SessionKind, SessionStatus};
use std::fs;
use std::process::{Child, Command};
use tracing::{info, warn};

fn resolve_uid(username: &str) -> Option<u32> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 && parts[0] == username {
            parts[2].parse::<u32>().ok()
        } else {
            None
        }
    })
}

fn resolve_home(username: &str) -> Option<String> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 6 && parts[0] == username {
            Some(parts[5].to_string())
        } else {
            None
        }
    })
}

fn binary_exists(name: &str) -> bool {
    let paths = ["/usr/bin", "/usr/local/bin", "/bin", "/usr/sbin", "/sbin"];
    for p in paths {
        if std::path::Path::new(p).join(name).exists() {
            return true;
        }
    }
    false
}

fn prepare_user_command(username: &str, display_str: &str) -> Command {
    let uid = resolve_uid(username).unwrap_or(1000);
    let home = resolve_home(username).unwrap_or_else(|| format!("/home/{}", username));
    let xdg_runtime = format!("/run/user/{}", uid);

    let mut cmd = Command::new("runuser");
    cmd.arg("-u").arg(username).arg("--");
    cmd.env("HOME", &home);
    cmd.env("USER", username);
    cmd.env("LOGNAME", username);
    cmd.env("XDG_RUNTIME_DIR", &xdg_runtime);
    cmd.env(
        "DBUS_SESSION_BUS_ADDRESS",
        format!("unix:path={}/bus", xdg_runtime),
    );
    cmd.env("DISPLAY", display_str);
    cmd.env_remove("WAYLAND_DISPLAY");
    cmd.env("XDG_SESSION_TYPE", "x11");
    cmd.env_remove("GDK_BACKEND");
    cmd.env_remove("QT_QPA_PLATFORM");
    cmd.env_remove("XAUTHORITY");
    cmd
}

fn detect_existing_session_kind(username: &str, display_id: u8) -> SessionKind {
    if let Some(uid) = resolve_uid(username) {
        let runtime_dir = std::path::PathBuf::from(format!("/run/user/{}", uid));
        if runtime_dir.join(format!("wayland-{}", display_id)).exists() {
            return SessionKind::Wayland;
        }
    }

    let x_socket = format!("/tmp/.X11-unix/X{}", display_id);
    if fs::metadata(&x_socket).is_ok() {
        SessionKind::X11
    } else {
        SessionKind::Unknown
    }
}

pub struct AstraX11UserSession {
    id: String,
    username: String,
    display_id: u8,
    session_kind: SessionKind,
    status: SessionStatus,
    xvfb_process: Option<Child>,
    desktop_process: Option<Child>,
}

impl AstraX11UserSession {
    pub fn start(username: &str, display_id: u8) -> Result<Self> {
        let display_str = format!(":{}", display_id);
        info!(
            "Starting Astra X11 session for {} on display {}",
            username, display_str
        );

        let mut session_kind = detect_existing_session_kind(username, display_id);

        let xvfb_proc = if matches!(session_kind, SessionKind::Wayland | SessionKind::X11) {
            None
        } else {
            let xvfb = Command::new("runuser")
                .arg("-u")
                .arg(username)
                .arg("--")
                .arg("Xvfb")
                .arg(&display_str)
                .arg("-screen")
                .arg("0")
                .arg("1920x1080x24")
                .arg("-nolisten")
                .arg("tcp")
                .spawn()
                .context("Failed to spawn Xvfb. Ensure Xvfb is installed.");

            match xvfb {
                Ok(c) => {
                    session_kind = SessionKind::Virtual;
                    Some(c)
                }
                Err(e) => {
                    warn!(
                        "Xvfb spawn failed: {}. Falling back to virtual session metadata only.",
                        e
                    );
                    session_kind = SessionKind::Virtual;
                    None
                }
            }
        };

        let desktop_proc = if matches!(session_kind, SessionKind::Virtual) && xvfb_proc.is_some() {
            let wms = vec![
                "fly-wm",
                "openbox",
                "kwin_x11",
                "mate-session",
                "xfce4-session",
                "i3",
                "xterm",
            ];
            let mut spawned = None;
            for wm in wms {
                if !binary_exists(wm) {
                    continue;
                }
                info!("Trying to spawn window manager: {}", wm);
                let child = prepare_user_command(username, &display_str).arg(wm).spawn();
                match child {
                    Ok(c) => {
                        info!("Successfully spawned window manager: {}", wm);
                        spawned = Some(c);
                        break;
                    }
                    Err(_) => {
                        warn!("Window manager {} is not available.", wm);
                    }
                }
            }

            if spawned.is_some() {
                info!(
                    "Spawning desktop helper applications on display {}",
                    display_str
                );

                if binary_exists("xsetroot") {
                    let _ = prepare_user_command(username, &display_str)
                        .arg("xsetroot")
                        .args(["-solid", "#1c1d26"])
                        .spawn();
                }

                let terminals = vec![
                    "konsole",
                    "x-terminal-emulator",
                    "mate-terminal",
                    "gnome-terminal",
                    "xterm",
                ];
                for term in terminals {
                    if !binary_exists(term) {
                        continue;
                    }
                    if prepare_user_command(username, &display_str)
                        .arg(term)
                        .spawn()
                        .is_ok()
                    {
                        info!("Successfully spawned terminal: {}", term);
                        break;
                    }
                }

                let file_managers = vec!["fly-fm", "pcmanfm", "thunar", "nautilus", "dolphin"];
                for fm in file_managers {
                    if !binary_exists(fm) {
                        continue;
                    }
                    if prepare_user_command(username, &display_str)
                        .arg(fm)
                        .spawn()
                        .is_ok()
                    {
                        info!("Successfully spawned file manager: {}", fm);
                        break;
                    }
                }
            }

            spawned
        } else {
            None
        };

        Ok(Self {
            id: format!("{}-astra-{}", username, display_id),
            username: username.to_string(),
            display_id,
            session_kind,
            status: SessionStatus::Active,
            xvfb_process: xvfb_proc,
            desktop_process: desktop_proc,
        })
    }
}

impl UserSession for AstraX11UserSession {
    fn id(&self) -> &str {
        &self.id
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn display_id(&self) -> u8 {
        self.display_id
    }

    fn session_kind(&self) -> SessionKind {
        self.session_kind
    }

    fn status(&self) -> SessionStatus {
        self.status
    }

    fn stop(&mut self) -> Result<()> {
        info!("Stopping Astra X11 session {}", self.id);

        if let Some(mut proc) = self.desktop_process.take() {
            let _ = proc.kill();
            let _ = proc.wait();
        }

        if let Some(mut proc) = self.xvfb_process.take() {
            let _ = proc.kill();
            let _ = proc.wait();
        }

        let lock_file = format!("/tmp/.X{}-lock", self.display_id);
        let socket_file = format!("/tmp/.X11-unix/X{}", self.display_id);
        let _ = fs::remove_file(lock_file);
        let _ = fs::remove_file(socket_file);

        self.status = SessionStatus::Disconnected;
        Ok(())
    }
}

pub struct AstraX11Backend;

impl SessionBackend for AstraX11Backend {
    fn create_session(&self, username: &str, display_id: u8) -> Result<Box<dyn UserSession>> {
        let session = AstraX11UserSession::start(username, display_id)?;
        Ok(Box::new(session))
    }
}
