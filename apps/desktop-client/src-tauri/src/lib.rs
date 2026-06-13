use base64::prelude::*;
use directories::ProjectDirs;
use protocol::messages::{InputEvent, KeyboardEvent, MouseEvent};
use protocol::Frame;
use shared_types::ConnectionConfig;
use shared_types::MouseButton;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, Mutex};
use transport::TcpTransport;

struct AppState {
    tx_frames: Mutex<Option<mpsc::Sender<Frame>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct SshIdentityInfo {
    public_key: String,
    public_key_path: String,
    private_key_path: String,
}

fn project_data_dir() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from("com", "TTGTiSO", "TTGTiSO-Desk")
        .ok_or_else(|| "Unable to determine application data directory".to_string())?;
    Ok(dirs.data_local_dir().to_path_buf())
}

fn ssh_identity_paths() -> Result<(PathBuf, PathBuf), String> {
    let base_dir = project_data_dir()?.join("ssh");
    Ok((base_dir.join("id_ed25519"), base_dir.join("id_ed25519.pub")))
}

fn load_ssh_identity() -> Result<SshIdentityInfo, String> {
    let (private_key_path, public_key_path) = ssh_identity_paths()?;
    let public_key = std::fs::read_to_string(&public_key_path).map_err(|e| {
        format!(
            "Failed to read public SSH key from {}: {}",
            public_key_path.display(),
            e
        )
    })?;

    Ok(SshIdentityInfo {
        public_key: public_key.trim().to_string(),
        public_key_path: public_key_path.to_string_lossy().to_string(),
        private_key_path: private_key_path.to_string_lossy().to_string(),
    })
}

fn generate_ssh_identity() -> Result<SshIdentityInfo, String> {
    let (private_key_path, public_key_path) = ssh_identity_paths()?;
    if let Some(parent) = private_key_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create SSH key directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    if private_key_path.exists() {
        let _ = std::fs::remove_file(&private_key_path);
    }
    if public_key_path.exists() {
        let _ = std::fs::remove_file(&public_key_path);
    }

    let status = StdCommand::new("ssh-keygen")
        .args([
            "-t",
            "ed25519",
            "-N",
            "",
            "-f",
            private_key_path.to_string_lossy().as_ref(),
            "-C",
            "TTGTiSO-Desk client",
            "-q",
        ])
        .status()
        .map_err(|e| format!("Failed to run ssh-keygen: {}", e))?;

    if !status.success() {
        return Err("ssh-keygen failed while creating SSH identity".to_string());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&private_key_path, std::fs::Permissions::from_mode(0o600));
    }

    load_ssh_identity()
}

fn ensure_ssh_identity_on_disk() -> Result<SshIdentityInfo, String> {
    let (private_key_path, public_key_path) = ssh_identity_paths()?;
    if private_key_path.exists() && public_key_path.exists() {
        return load_ssh_identity();
    }
    generate_ssh_identity()
}

fn api_server_port_open() -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    TcpStream::connect_timeout(&addr, Duration::from_millis(250)).is_ok()
}

fn ensure_api_server_running() {
    let _ = ensure_ssh_identity_on_disk();
    if api_server_port_open() {
        return;
    }

    if std::env::var("TTGTISO_HOSTS_TOML_PATH").is_err() {
        if let Ok(mut dir) = std::env::current_dir() {
            if dir.ends_with("src-tauri") {
                dir.pop();
            }
            if dir.ends_with("desktop-client") {
                dir.pop();
            }
            if dir.ends_with("apps") {
                dir.pop();
            }
            let hosts_path = dir.join("hosts.toml");
            std::env::set_var("TTGTISO_HOSTS_TOML_PATH", hosts_path);
        }
    }

    tauri::async_runtime::spawn(async move {
        if let Err(e) = api_server::run().await {
            eprintln!("api-server stopped: {e}");
        }
    });
}

#[tauri::command]
async fn connect_to_agent(
    app: AppHandle,
    host: String,
    port: u16,
    username: String,
    display_id: Option<u32>,
    connection_token: Option<String>,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    {
        let mut tx_lock = state.tx_frames.lock().await;
        *tx_lock = None;
    }

    let config = ConnectionConfig {
        host: host.clone(),
        port,
        username: "client".to_string(),
        auth_method: shared_types::AuthMethod::Agent,
    };

    let mut transport = TcpTransport::new(config);
    if let Err(e) = transport.connect().await {
        return Err(format!("Failed to connect: {}", e));
    }

    let handshake_payload = serde_json::json!({
        "session_id": "s1",
        "username": username,
        "display_id": display_id,
        "connection_token": connection_token,
    })
    .to_string();

    let handshake = Frame {
        header: protocol::FrameHeader {
            version: 1,
            channel_id: 0,
            length: handshake_payload.len() as u32,
            flags: 0x00,
        },
        payload: handshake_payload.into_bytes(),
    };
    if let Err(e) = transport.send_frame(handshake).await {
        return Err(format!("Failed to send handshake: {}", e));
    }

    let ack = transport
        .receive_frame()
        .await
        .map_err(|e| format!("Failed to receive handshake ack: {}", e))?;
    if ack.header.channel_id != 0 || ack.payload.as_slice() != b"OK" {
        return Err("Invalid handshake ack from server-agent".to_string());
    }

    let (tx, mut rx) = mpsc::channel::<Frame>(100);
    {
        let mut tx_lock = state.tx_frames.lock().await;
        *tx_lock = Some(tx);
    }

    tokio::spawn(async move {
        loop {
            tokio::select! {
                frame_res = transport.receive_frame() => {
                    match frame_res {
                        Ok(frame) => {
                            if frame.header.channel_id == 1 {
                                let b64 = BASE64_STANDARD.encode(&frame.payload);
                                let _ = app.emit("video_frame", b64);
                            }
                        }
                        Err(e) => {
                            println!("Transport read error: {}", e);
                            break;
                        }
                    }
                }
                out_frame = rx.recv() => {
                    if let Some(frame) = out_frame {
                        if transport.send_frame(frame).await.is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        println!("Connection loop closed.");
        let _ = transport.disconnect().await;
        let _ = app.emit("connection_closed", ());
    });

    Ok(true)
}

#[tauri::command]
async fn disconnect_agent(state: State<'_, AppState>) -> Result<(), String> {
    let mut tx_lock = state.tx_frames.lock().await;
    *tx_lock = None;
    Ok(())
}

#[tauri::command]
async fn send_mouse_event(
    event_type: u8,
    button: u8,
    x: u16,
    y: u16,
    scroll_delta: i16,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let btn = match button {
        1 => MouseButton::Left,
        2 => MouseButton::Right,
        3 => MouseButton::Middle,
        _ => MouseButton::None,
    };

    let ev = InputEvent::Mouse(MouseEvent {
        event_type,
        button: btn,
        x,
        y,
        scroll_delta,
    });

    let payload = ev.to_bytes();
    let frame = Frame {
        header: protocol::FrameHeader {
            version: 1,
            channel_id: 2,
            length: payload.len() as u32,
            flags: 0x00,
        },
        payload,
    };

    let tx_lock = state.tx_frames.lock().await;
    if let Some(tx) = &*tx_lock {
        let _ = tx.send(frame).await;
    }
    Ok(())
}

#[tauri::command]
async fn send_keyboard_event(
    event_type: u8,
    keysym: u32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ev = InputEvent::Keyboard(KeyboardEvent { event_type, keysym });

    let payload = ev.to_bytes();
    let frame = Frame {
        header: protocol::FrameHeader {
            version: 1,
            channel_id: 2,
            length: payload.len() as u32,
            flags: 0x00,
        },
        payload,
    };

    let tx_lock = state.tx_frames.lock().await;
    if let Some(tx) = &*tx_lock {
        let _ = tx.send(frame).await;
    }
    Ok(())
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn ensure_ssh_identity() -> Result<SshIdentityInfo, String> {
    ensure_ssh_identity_on_disk()
}

#[tauri::command]
fn regenerate_ssh_identity() -> Result<SshIdentityInfo, String> {
    generate_ssh_identity()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
        }
        if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            }
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            tx_frames: Mutex::new(None),
        })
        .setup(|_| {
            ensure_api_server_running();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            connect_to_agent,
            disconnect_agent,
            send_mouse_event,
            send_keyboard_event,
            get_app_version,
            ensure_ssh_identity,
            regenerate_ssh_identity
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
