use anyhow::{Context, Result};
use std::process::{Command, Child};
use std::fs;
use shared_types::SessionStatus;
use crate::traits::{SessionBackend, UserSession};
use tracing::{info, warn};

pub struct AstraX11UserSession {
    id: String,
    username: String,
    display_id: u8,
    status: SessionStatus,
    xvfb_process: Option<Child>,
    desktop_process: Option<Child>,
}

impl AstraX11UserSession {
    pub fn start(username: &str, display_id: u8) -> Result<Self> {
        let display_str = format!(":{}", display_id);
        info!("Starting Astra X11 session for {} on display {}", username, display_str);

        // 1. Spawn Xvfb process
        let xvfb = Command::new("runuser")
            .arg("-u").arg(username)
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

        let xvfb_proc = match xvfb {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("Xvfb spawn failed: {}. Running in mock-Astra mode.", e);
                None
            }
        };

        // 2. Spawn Window Manager / Desktop (supporting Astra, Alt, and Arch Linux)
        let desktop_proc = if xvfb_proc.is_some() {
            let wms = vec!["fly-wm", "openbox", "mate-session", "xfce4-session", "i3", "xterm"];
            let mut spawned = None;
            let mut spawned_name = None;
            for wm in wms {
                info!("Trying to spawn window manager: {}", wm);
                let child = Command::new("runuser")
                    .arg("-u").arg(username)
                    .arg("--")
                    .arg(wm)
                    .env("DISPLAY", &display_str)
                    .env_remove("WAYLAND_DISPLAY")
                    .env_remove("XDG_SESSION_TYPE")
                    .env_remove("GDK_BACKEND")
                    .env_remove("QT_QPA_PLATFORM")
                    .spawn();
                match child {
                    Ok(c) => {
                        info!("Successfully spawned window manager: {}", wm);
                        spawned = Some(c);
                        spawned_name = Some(wm.to_string());
                        break;
                    }
                    Err(_) => {
                        warn!("Window manager {} is not available.", wm);
                    }
                }
            }

            if let Some(ref _name) = spawned_name {
                info!("Spawning desktop helper applications on display {}", display_str);
                
                // Set solid dark slate background color
                let _ = Command::new("runuser")
                    .arg("-u").arg(username)
                    .arg("--")
                    .arg("xsetroot")
                    .env("DISPLAY", &display_str)
                    .args(["-solid", "#1c1d26"])
                    .spawn();

                // Spawn a terminal emulator
                let terminals = vec!["konsole", "x-terminal-emulator", "mate-terminal", "gnome-terminal", "xterm"];
                for term in terminals {
                    if Command::new("runuser")
                        .arg("-u").arg(username)
                        .arg("--")
                        .arg(term)
                        .env("DISPLAY", &display_str)
                        .env_remove("WAYLAND_DISPLAY")
                        .env_remove("XDG_SESSION_TYPE")
                        .env_remove("GDK_BACKEND")
                        .env_remove("QT_QPA_PLATFORM")
                        .spawn()
                        .is_ok()
                    {
                        info!("Successfully spawned terminal: {}", term);
                        break;
                    }
                }
                // Spawn a file manager
                let file_managers = vec!["fly-fm", "pcmanfm", "thunar", "nautilus", "dolphin"];
                for fm in file_managers {
                    if Command::new("runuser")
                        .arg("-u").arg(username)
                        .arg("--")
                        .arg(fm)
                        .env("DISPLAY", &display_str)
                        .env_remove("WAYLAND_DISPLAY")
                        .env_remove("XDG_SESSION_TYPE")
                        .env_remove("GDK_BACKEND")
                        .env_remove("QT_QPA_PLATFORM")
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

        // Clean up X11 files
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
