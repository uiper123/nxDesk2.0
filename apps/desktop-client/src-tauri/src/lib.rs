use base64::prelude::*;
use protocol::messages::{InputEvent, KeyboardEvent, MouseEvent};
use protocol::Frame;
use shared_types::ConnectionConfig;
use shared_types::MouseButton;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, Mutex};
use transport::TcpTransport;

struct AppState {
    tx_frames: Mutex<Option<mpsc::Sender<Frame>>>,
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
    // Clean up previous connection if any
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

    // Send handshake
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

    // Spawn relay loop
    tokio::spawn(async move {
        loop {
            tokio::select! {
                frame_res = transport.receive_frame() => {
                    match frame_res {
                        Ok(frame) => {
                            // If video frame, emit to frontend
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
    *tx_lock = None; // Drops the sender, breaking the loop
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            tx_frames: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            connect_to_agent,
            disconnect_agent,
            send_mouse_event,
            send_keyboard_event,
            get_app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
