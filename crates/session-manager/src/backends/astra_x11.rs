use crate::traits::{SessionBackend, SessionProvisioning, UserSession};
use anyhow::{Context, Result};
use config::{ConnectionMode, DesktopSessionType};
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
    pub fn start(username: &str, display_id: Option<u8>) -> Result<Self> {
        let display_id = display_id.unwrap_or(10);
        let display_str = format!(":{}", display_id);
        info!("Starting desktop session for {} on {}", username, display_str);

        let session_kind = detect_existing_session_kind(username, display_id);
        let mut xvfb_proc = None;
        let mut desktop_proc = None;

        match session_kind {
            SessionKind::X11 | SessionKind::Wayland => {
                info!("Attaching to existing graphical session {} for {}", display_str, username);
            }
            _ => {
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
                        xvfb_proc = Some(c);
                        let shells = [
                            "fly-wm",
                            "startplasma-x11",
                            "plasma-session",
                            "gnome-session",
                            "xfce4-session",
                            "mate-session",
                            "cinnamon-session",
                            "openbox",
                        ];
                        for shell in shells {
                            if !binary_exists(shell) {
                                continue;
                            }
                            if let Ok(child) = prepare_user_command(username, &display_str).arg(shell).spawn() {
                                desktop_proc = Some(child);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Xvfb spawn failed: {}", e);
                    }
                }
            }
        }

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
    fn id(&self) -> &str { &self.id }
    fn username(&self) -> &str { &self.username }
    fn display_id(&self) -> u8 { self.display_id }
    fn session_kind(&self) -> SessionKind { self.session_kind }
    fn status(&self) -> SessionStatus { self.status }
    fn stop(&mut self) -> Result<()> {
        if let Some(mut proc) = self.desktop_process.take() { let _ = proc.kill(); let _ = proc.wait(); }
        if let Some(mut proc) = self.xvfb_process.take() { let _ = proc.kill(); let _ = proc.wait(); }
        let _ = fs::remove_file(format!("/tmp/.X{}-lock", self.display_id));
        let _ = fs::remove_file(format!("/tmp/.X11-unix/X{}", self.display_id));
        self.status = SessionStatus::Disconnected;
        Ok(())
    }
}

pub struct AstraX11Backend {
    mode: ConnectionMode,
    session_type: DesktopSessionType,
}

impl AstraX11Backend {
    pub fn from_config(mode: ConnectionMode, session_type: DesktopSessionType) -> Self {
        Self { mode, session_type }
    }

    fn should_attach(&self) -> bool {
        matches!(self.session_type, DesktopSessionType::Attach)
            || matches!(self.session_type, DesktopSessionType::Auto)
                && matches!(self.mode, ConnectionMode::App)
    }
}

impl Default for AstraX11Backend {
    fn default() -> Self {
        Self { mode: ConnectionMode::Desktop, session_type: DesktopSessionType::Auto }
    }
}

impl SessionBackend for AstraX11Backend {
    fn provisioning(&self, _username: &str) -> SessionProvisioning {
        if self.should_attach() {
            SessionProvisioning::AttachExisting
        } else {
            SessionProvisioning::VirtualDesktop
        }
    }

    fn create_session(&self, username: &str, display_id: Option<u8>) -> Result<Box<dyn UserSession>> {
        Ok(Box::new(AstraX11UserSession::start(username, display_id)?))
    }
}
