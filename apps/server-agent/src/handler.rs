use audit::AuditLog;
use input_injector::LegacyX11InputInjector;
use protocol::messages::InputEvent;
use session_manager::{LocalSessionManager, SessionManager};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use video_pipeline::traits::VideoStream;
use video_pipeline::{
    make_capture_source, LocalVideoStream, SimpleFrameClock, SoftwareFallbackEncoder,
};

/// Global connection statistics
static TOTAL_CONNECTIONS: AtomicU64 = AtomicU64::new(0);
static ACTIVE_CONNECTIONS: AtomicU64 = AtomicU64::new(0);
static TOTAL_BYTES_RECEIVED: AtomicU64 = AtomicU64::new(0);
static TOTAL_BYTES_SENT: AtomicU64 = AtomicU64::new(0);

/// Get connection statistics as JSON
pub fn connection_stats() -> serde_json::Value {
    serde_json::json!({
        "total_connections": TOTAL_CONNECTIONS.load(Ordering::Relaxed),
        "active_connections": ACTIVE_CONNECTIONS.load(Ordering::Relaxed),
        "total_bytes_received": TOTAL_BYTES_RECEIVED.load(Ordering::Relaxed),
        "total_bytes_sent": TOTAL_BYTES_SENT.load(Ordering::Relaxed),
    })
}

pub async fn run_connection_listener(
    port: u16,
    session_mgr: Arc<LocalSessionManager>,
    audit_log: Arc<AuditLog>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            info!("Connection listener bound to {}", addr);
            l
        }
        Err(_) => {
            let fallback_addr = "0.0.0.0:2222";
            match TcpListener::bind(fallback_addr).await {
                Ok(l) => {
                    warn!(
                        "Could not bind to {}, falling back to {}",
                        addr, fallback_addr
                    );
                    l
                }
                Err(e) => {
                    error!("Failed to bind listener to fallback address: {}", e);
                    return;
                }
            }
        }
    };

    loop {
        tokio::select! {
            accept_res = listener.accept() => {
                match accept_res {
                    Ok((stream, peer_addr)) => {
                        TOTAL_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
                        ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);

                        let peer_ip = peer_addr.ip().to_string();
                        info!("Accepted connection from client: {}", peer_addr);
                        audit_log.log_auth_success("incoming_connection", &peer_ip);

                        let session_mgr = session_mgr.clone();
                        let audit_log = audit_log.clone();

                        tokio::spawn(async move {
                            handle_client_connection(stream, peer_addr, session_mgr, audit_log).await;
                            ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(e) => {
                        warn!("TCP accept error: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Stopping connection listener...");
                break;
            }
        }
    }
}

async fn handle_client_connection(
    mut socket: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    session_mgr: Arc<LocalSessionManager>,
    audit_log: Arc<AuditLog>,
) {
    let peer_ip = peer_addr.ip().to_string();
    let connection_start = std::time::Instant::now();
    let mut bytes_received: u64 = 0;
    let mut bytes_sent: u64 = 0;

    let mut header_buf = [0u8; 11];
    if let Err(e) = socket.read_exact(&mut header_buf).await {
        warn!("Failed to read handshake header from {}: {}", peer_addr, e);
        return;
    }
    bytes_received += 11;
    TOTAL_BYTES_RECEIVED.fetch_add(11, Ordering::Relaxed);

    if &header_buf[0..4] != b"TTGT" {
        warn!(
            "Invalid magic bytes from {}. Closing connection.",
            peer_addr
        );
        audit_log.log_auth_failure("unknown", &peer_ip, "Invalid protocol magic bytes");
        return;
    }

    let protocol_version = header_buf[4];
    let channel_id = header_buf[5];
    let payload_length =
        u32::from_be_bytes([header_buf[6], header_buf[7], header_buf[8], header_buf[9]]) as usize;

    info!(
        "Handshake from {}: version={}, channel={}, payload_len={}",
        peer_addr, protocol_version, channel_id, payload_length
    );

    let mut target_username = None;
    let mut target_display_id = None;

    if payload_length > 0 && payload_length < 65536 {
        let mut payload = vec![0u8; payload_length];
        if let Err(e) = socket.read_exact(&mut payload).await {
            warn!("Failed to read handshake payload from {}: {}", peer_addr, e);
            return;
        }
        bytes_received += payload_length as u64;
        TOTAL_BYTES_RECEIVED.fetch_add(payload_length as u64, Ordering::Relaxed);

        if let Ok(text) = std::str::from_utf8(&payload) {
            info!("Handshake payload from {}: {}", peer_addr, text);
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(text) {
                if let Some(user) = json_val.get("username").and_then(|u| u.as_str()) {
                    target_username = Some(user.to_string());
                }
                if let Some(display_id) = json_val.get("display_id").and_then(|d| d.as_u64()) {
                    target_display_id = Some(display_id as u8);
                }
            }
        }
    }

    let ack_payload = b"OK";
    let ack_frame = build_frame(protocol_version, channel_id, 0x01, ack_payload);
    if let Err(e) = socket.write_all(&ack_frame).await {
        warn!("Failed to send ack to {}: {}", peer_addr, e);
        return;
    }
    bytes_sent += ack_frame.len() as u64;
    TOTAL_BYTES_SENT.fetch_add(ack_frame.len() as u64, Ordering::Relaxed);

    // Pick active session matching target_username or fallback to first active session
    let active_sessions = session_mgr.list_active_sessions().unwrap_or_default();
    if active_sessions.is_empty() {
        warn!(
            "No active sessions found for {}. Please start a session via API first.",
            peer_addr
        );
    }

    let default_display = std::env::var("DISPLAY")
        .ok()
        .and_then(|val| {
            val.trim_start_matches(':')
                .split('.')
                .next()
                .and_then(|s| s.parse::<u8>().ok())
        })
        .unwrap_or(10);

    let display_id = if let Some(display_id) = target_display_id {
        display_id
    } else if let Some(ref username) = target_username {
        active_sessions
            .iter()
            .find(|s| &s.username == username)
            .map(|s| s.display_id)
            .unwrap_or_else(|| {
                warn!(
                    "No active session found for username: {}. Falling back to first available.",
                    username
                );
                active_sessions
                    .first()
                    .map(|s| s.display_id)
                    .unwrap_or(default_display)
            })
    } else {
        active_sessions
            .first()
            .map(|s| s.display_id)
            .unwrap_or(default_display)
    };
    let display_str = format!(":{}", display_id);
    info!(
        "Attaching connection from {} to display {} (user={:?}, requested_display={:?})",
        peer_addr, display_str, target_username, target_display_id
    );

    // Initialize Input Injector
    let injector = LegacyX11InputInjector::new(&display_str);

    let (rx_socket, mut tx_socket) = socket.into_split();

    // Spawn Video Streaming Task
    let display_clone = display_str.clone();
    let video_task = tokio::spawn(async move {
        // Start streaming 15 FPS
        let capture = make_capture_source(&display_clone, 1920, 1080);
        let encoder = Box::new(SoftwareFallbackEncoder::new(2000));
        let clock = Box::new(SimpleFrameClock::new(15));
        let mut stream = LocalVideoStream::new(capture, encoder, clock);
        let mut frames_sent = 0u64;

        loop {
            // Encode frame (this blocks, so ideally it should be inside spawn_blocking,
            // but for MVP we run it here)
            let frame_res = tokio::task::spawn_blocking(move || {
                let frame = stream.next_frame();
                (frame, stream)
            })
            .await;

            match frame_res {
                Ok((Ok(frame), returned_stream)) => {
                    stream = returned_stream;
                    let out_frame = build_frame(
                        1,
                        1,
                        if frame.is_keyframe { 0x01 } else { 0x00 },
                        &frame.data,
                    );
                    if tx_socket.write_all(&out_frame).await.is_err() {
                        break; // Connection closed
                    }
                    frames_sent += 1;
                    if frames_sent == 1 {
                        info!(
                            "Sent first video frame from display {} ({} bytes)",
                            display_clone,
                            frame.data.len()
                        );
                    }
                    TOTAL_BYTES_SENT.fetch_add(out_frame.len() as u64, Ordering::Relaxed);
                }
                Ok((Err(e), returned_stream)) => {
                    stream = returned_stream;
                    warn!("Video frame encoding error: {:?}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
                Err(_) => break,
            }
        }
    });

    let mut reader = tokio::io::BufReader::new(rx_socket);
    loop {
        let mut header = [0u8; 11];
        if reader.read_exact(&mut header).await.is_err() {
            info!("Connection with {} closed by client", peer_addr);
            break;
        }

        if &header[0..4] != b"TTGT" {
            warn!(
                "Invalid magic bytes from {}. Closing connection.",
                peer_addr
            );
            break;
        }

        let ch = header[5];
        let payload_len = u32::from_be_bytes([header[6], header[7], header[8], header[9]]) as usize;

        let mut payload = vec![0u8; payload_len];
        if reader.read_exact(&mut payload).await.is_err() {
            warn!("Connection with {} closed mid-frame", peer_addr);
            break;
        }

        bytes_received += 11 + payload_len as u64;
        TOTAL_BYTES_RECEIVED.fetch_add(11 + payload_len as u64, Ordering::Relaxed);

        if ch == 2 {
            // Input channel
            if let Ok(event) = InputEvent::from_bytes(&payload) {
                match event {
                    InputEvent::Mouse(m) => {
                        if m.event_type == 0x01 {
                            if let Err(e) = injector.inject_mouse_move(m.x, m.y) {
                                warn!("Failed to inject mouse move: {:?}", e);
                            }
                        } else if m.event_type == 0x02 || m.event_type == 0x03 {
                            if let Err(e) =
                                injector.inject_mouse_click(m.button, m.event_type == 0x02)
                            {
                                warn!("Failed to inject mouse click: {:?}", e);
                            }
                        } else if m.event_type == 0x04 {
                            if let Err(e) = injector.inject_mouse_scroll(m.scroll_delta) {
                                warn!("Failed to inject mouse scroll: {:?}", e);
                            }
                        }
                    }
                    InputEvent::Keyboard(k) => {
                        if let Err(e) = injector.inject_keypress(k.keysym, k.event_type == 0x05) {
                            warn!("Failed to inject keypress: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    video_task.abort();

    let duration = connection_start.elapsed();
    info!(
        "Connection from {} closed. Duration: {:?}, Bytes received: {}, Bytes sent: {}",
        peer_addr, duration, bytes_received, bytes_sent
    );

    audit_log.write_record(audit::AuditRecord {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        event_type: "CONNECTION".to_string(),
        username: "client".to_string(),
        ip_address: peer_ip,
        action: "connection_closed".to_string(),
        details: format!(
            "Duration: {:.1}s, Received: {} bytes, Sent: {} bytes",
            duration.as_secs_f64(),
            bytes_received,
            bytes_sent
        ),
    });
}

fn build_frame(version: u8, channel_id: u8, flags: u8, payload: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(11 + payload.len());
    buffer.extend_from_slice(b"TTGT");
    buffer.push(version);
    buffer.push(channel_id);
    buffer.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    buffer.push(flags);
    buffer.extend_from_slice(payload);
    buffer
}
