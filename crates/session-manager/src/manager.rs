use crate::traits::{DisplayAllocator, SessionBackend, SessionManager, UserSession};
use anyhow::{bail, Result};
use shared_types::{SessionInfo, SessionStatus};
#[cfg(unix)]
use shared_types::SessionKind;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct ManagedSession {
    session: Box<dyn UserSession>,
    start_time: u64,
}

pub struct LocalSessionManager {
    backend: Box<dyn SessionBackend>,
    allocator: Arc<dyn DisplayAllocator>,
    sessions: Arc<Mutex<HashMap<String, ManagedSession>>>,
}

impl LocalSessionManager {
    pub fn new(backend: Box<dyn SessionBackend>, allocator: Arc<dyn DisplayAllocator>) -> Self {
        Self {
            backend,
            allocator,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl SessionManager for LocalSessionManager {
    fn start_session(&self, username: &str) -> Result<SessionInfo> {
        let mut sessions = self.sessions.lock().unwrap();

        // Prevent duplicate active sessions for the same user
        for managed in sessions.values() {
            if managed.session.username() == username
                && managed.session.status() == SessionStatus::Active
            {
                bail!("User {} already has an active session", username);
            }
        }

        // Allocate a display
        let display_id = self.allocator.allocate()?;

        // Create session
        let session = match self.backend.create_session(username, display_id) {
            Ok(s) => s,
            Err(e) => {
                self.allocator.release(display_id);
                return Err(e);
            }
        };

        let start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let info = SessionInfo {
            id: session.id().to_string(),
            username: session.username().to_string(),
            display_id,
            start_time,
            session_kind: session.session_kind(),
        };

        sessions.insert(
            info.id.clone(),
            ManagedSession {
                session,
                start_time,
            },
        );
        Ok(info)
    }

    fn stop_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(mut managed) = sessions.remove(session_id) {
            let display_id = managed.session.display_id();
            managed.session.stop()?;
            self.allocator.release(display_id);
            Ok(())
        } else {
            bail!("Session not found: {}", session_id)
        }
    }

    fn get_session_info(&self, session_id: &str) -> Result<SessionInfo> {
        let sessions = self.sessions.lock().unwrap();
        if let Some(managed) = sessions.get(session_id) {
            Ok(SessionInfo {
                id: managed.session.id().to_string(),
                username: managed.session.username().to_string(),
                display_id: managed.session.display_id(),
                start_time: managed.start_time,
                session_kind: managed.session.session_kind(),
            })
        } else {
            bail!("Session not found: {}", session_id)
        }
    }

    fn list_active_sessions(&self) -> Result<Vec<SessionInfo>> {
        let sessions = self.sessions.lock().unwrap();
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut list: Vec<SessionInfo> = sessions
            .values()
            .map(|managed| SessionInfo {
                id: managed.session.id().to_string(),
                username: managed.session.username().to_string(),
                display_id: managed.session.display_id(),
                start_time: managed.start_time,
                session_kind: managed.session.session_kind(),
            })
            .collect();

        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::MetadataExt;

            let resolve_username = |uid: u32| -> Option<String> {
                let passwd = fs::read_to_string("/etc/passwd").ok()?;
                for line in passwd.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 3 {
                        if let Ok(parsed_uid) = parts[2].parse::<u32>() {
                            if parsed_uid == uid {
                                return Some(parts[0].to_string());
                            }
                        }
                    }
                }
                None
            };

            if let Ok(entries) = fs::read_dir("/tmp/.X11-unix") {
                for entry in entries.flatten() {
                    if let Some(filename) = entry.file_name().to_str() {
                        if let Some(stripped) = filename.strip_prefix('X') {
                            if let Ok(display_num) = stripped.parse::<u8>() {
                                if list.iter().any(|s| s.display_id == display_num) {
                                    continue;
                                }

                                let username = if let Ok(meta) = entry.metadata() {
                                    let uid = meta.uid();
                                    if uid == 0 {
                                        "root (DM/System)".to_string()
                                    } else {
                                        resolve_username(uid)
                                            .unwrap_or_else(|| format!("user_{}", uid))
                                    }
                                } else {
                                    "unknown".to_string()
                                };

                                list.push(SessionInfo {
                                    id: format!("system-display-{}", display_num),
                                    username,
                                    display_id: display_num,
                                    start_time: 0,
                                    session_kind: SessionKind::X11,
                                });
                            }
                        }
                    }
                }
            }

            // Also scan for Wayland sessions in /run/user/
            if let Ok(entries) = fs::read_dir("/run/user") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let uid_str = entry.file_name().to_string_lossy().to_string();
                        if let Ok(uid) = uid_str.parse::<u32>() {
                            if let Ok(user_entries) = fs::read_dir(&path) {
                                for ue in user_entries.flatten() {
                                    if let Some(fname) = ue.file_name().to_str() {
                                        if fname.starts_with("wayland-") && !fname.contains('.') {
                                            if let Ok(disp_num) =
                                                fname["wayland-".len()..].parse::<u8>()
                                            {
                                                let username = resolve_username(uid)
                                                    .unwrap_or_else(|| format!("user_{}", uid));

                                                // If we already have a root DM session on this display, remove it in favor of the real user session
                                                if let Some(pos) = list.iter().position(|s| {
                                                    s.display_id == disp_num
                                                        && s.username == "root (DM/System)"
                                                }) {
                                                    list.remove(pos);
                                                }

                                                // Only add if not already present
                                                if !list.iter().any(|s| s.display_id == disp_num) {
                                                    list.push(SessionInfo {
                                                        id: format!("system-wayland-{}", disp_num),
                                                        username: username.clone(),
                                                        display_id: disp_num,
                                                        start_time: 0,
                                                        session_kind: SessionKind::Wayland,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(list)
    }
}
