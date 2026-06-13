use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

use crate::models::*;
use crate::state::AppState;

fn generate_session_token() -> String {
    #[cfg(unix)]
    {
        if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
            use std::io::Read;
            let mut buf = [0u8; 16];
            if file.read_exact(&mut buf).is_ok() {
                return buf.iter().map(|b| format!("{:02x}", b)).collect();
            }
        }
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

pub async fn health_check() -> &'static str {
    "OK"
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    info!("Admin login attempt for user: {}", payload.username);

    let security_mgr = security::SecurityManager::new();
    match security_mgr.authenticate(&payload.username, &payload.password) {
        Ok(role) => {
            let role_str = match role {
                security::UserRole::Admin => "Administrator".to_string(),
                security::UserRole::SupportOperator => "Operator".to_string(),
                security::UserRole::Auditor => "Auditor".to_string(),
                security::UserRole::User => "User".to_string(),
            };

            let token = generate_session_token();
            state.active_tokens.write().await.insert(token.clone());

            Ok(Json(LoginResponse {
                success: true,
                message: "Authentication successful".to_string(),
                user: Some(UserInfo {
                    username: payload.username,
                    role: role_str,
                    token,
                }),
            }))
        }
        Err(e) => Ok(Json(LoginResponse {
            success: false,
            message: format!("Authentication failed: {}", e),
            user: None,
        })),
    }
}

pub async fn start_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    info!("Starting remote session for user: {}", payload.username);

    if !payload.username.is_empty() && !payload.host.is_empty() {
        let mut port = 22;
        {
            let hosts = state.hosts.read().await;
            if let Some(h) = hosts.iter().find(|h| h.ip == payload.host) {
                port = h.port;
            }
        }

        match state
            .discovery
            .start_session_on_host(&payload.host, port, &payload.username)
            .await
        {
            Ok(session) => {
                info!(
                    "Successfully started X11 session for {} on {}",
                    payload.username, payload.host
                );
                let mut sessions = state.sessions.write().await;
                sessions.push(session);

                Ok(Json(LoginResponse {
                    success: true,
                    message: "Session started".to_string(),
                    user: Some(UserInfo {
                        username: payload.username,
                        role: "Operator".to_string(),
                        token: "".to_string(),
                    }),
                }))
            }
            Err(e) => {
                let mut session_active = false;
                {
                    let sessions = state.sessions.read().await;
                    if sessions
                        .iter()
                        .any(|s| s.host_ip == payload.host && s.username == payload.username)
                    {
                        session_active = true;
                    }
                }

                if session_active {
                    Ok(Json(LoginResponse {
                        success: true,
                        message: "Session already active".to_string(),
                        user: Some(UserInfo {
                            username: payload.username,
                            role: "Operator".to_string(),
                            token: "".to_string(),
                        }),
                    }))
                } else {
                    tracing::warn!("Could not start session: {}", e);
                    Ok(Json(LoginResponse {
                        success: false,
                        message: format!("Failed to establish session: {}", e),
                        user: None,
                    }))
                }
            }
        }
    } else {
        Ok(Json(LoginResponse {
            success: false,
            message: "Invalid payload".to_string(),
            user: None,
        }))
    }
}

pub async fn get_hosts(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Host>>, StatusCode> {
    let hosts = state.hosts.read().await;
    Ok(Json(hosts.clone()))
}

#[derive(serde::Deserialize)]
pub struct AddHostRequest {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub ssh_public_key: Option<String>,
    pub ssh_public_key_path: Option<String>,
    pub ssh_private_key_path: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct UpdateHostRequest {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub ssh_public_key: Option<String>,
    pub ssh_public_key_path: Option<String>,
    pub ssh_private_key_path: Option<String>,
}

pub async fn get_discovered_hosts(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Host>>, StatusCode> {
    let hosts = state.discovered_hosts.read().await;
    Ok(Json(hosts.clone()))
}

fn build_host_from_request(id: String, payload: &AddHostRequest) -> Host {
    Host {
        id,
        name: payload.name.clone(),
        ip: payload.ip.clone(),
        port: payload.port,
        status: crate::models::HostStatus::Offline,
        active_sessions: 0,
        operating_system: "Unknown".to_string(),
        ssh_public_key: payload.ssh_public_key.clone(),
        ssh_public_key_path: payload.ssh_public_key_path.clone(),
        ssh_private_key_path: payload.ssh_private_key_path.clone(),
    }
}

fn rewrite_hosts_toml(hosts: &[Host]) {
    let mut text = String::from("# Конфигурация хостов TTGTiSO-Desk\n# Список серверов, на которых развёрнут server-agent\n\n");
    for host in hosts {
        text.push_str("[[hosts]]\n");
        text.push_str(&format!("name = \"{}\"\n", host.name));
        text.push_str(&format!("ip = \"{}\"\n", host.ip));
        text.push_str(&format!("ssh_port = {}\n", host.port));
        if let Some(key) = &host.ssh_public_key {
            text.push_str(&format!("ssh_public_key = \"{}\"\n", key.replace('"', "\\\"")));
        }
        if let Some(path) = &host.ssh_public_key_path {
            text.push_str(&format!("ssh_public_key_path = \"{}\"\n", path.replace('"', "\\\"")));
        }
        if let Some(path) = &host.ssh_private_key_path {
            text.push_str(&format!("ssh_private_key_path = \"{}\"\n", path.replace('"', "\\\"")));
        }
        text.push('\n');
    }
    let _ = std::fs::write("hosts.toml", text);
}

pub async fn add_host(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddHostRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut hosts = state.hosts.write().await;

    if hosts.iter().any(|h| h.ip == payload.ip) {
        return Ok(Json(serde_json::json!({
            "success": false,
            "message": "Host with this IP already exists in registry"
        })));
    }

    let next_id = (hosts.len() + 1).to_string();
    hosts.push(build_host_from_request(next_id, &payload));
    rewrite_hosts_toml(&hosts);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Host added"
    })))
}

pub async fn update_host(
    State(state): State<Arc<AppState>>,
    Path(ip): Path<String>,
    Json(payload): Json<UpdateHostRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut hosts = state.hosts.write().await;
    let Some(host) = hosts.iter_mut().find(|host| host.ip == ip) else {
        return Err(StatusCode::NOT_FOUND);
    };

    host.name = payload.name;
    host.ip = payload.ip;
    host.port = payload.port;
    host.ssh_public_key = payload.ssh_public_key;
    host.ssh_public_key_path = payload.ssh_public_key_path;
    host.ssh_private_key_path = payload.ssh_private_key_path;

    rewrite_hosts_toml(&hosts);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Host updated"
    })))
}

pub async fn get_active_sessions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ActiveSession>>, StatusCode> {
    let sessions = state.sessions.read().await;
    Ok(Json(sessions.clone()))
}

pub async fn terminate_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Terminating session: {}", session_id);

    let host_ip = {
        let sessions = state.sessions.read().await;
        sessions
            .iter()
            .find(|s| s.id == session_id)
            .map(|s| s.host_ip.clone())
    };

    if let Some(ip) = host_ip {
        let mut port = 22;
        {
            let hosts = state.hosts.read().await;
            if let Some(h) = hosts.iter().find(|h| h.ip == ip) {
                port = h.port;
            }
        }
        if let Err(e) = state.discovery.stop_session_on_host(&ip, port, &session_id).await {
            tracing::error!("Failed to stop session {} on {}: {}", session_id, ip, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        let mut sessions = state.sessions.write().await;
        sessions.retain(|s| s.id != session_id);

        Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Session {} terminated on {}", session_id, ip)
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn get_logs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    let logs = state.logs.read().await;
    Ok(Json(logs.clone()))
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Settings>, StatusCode> {
    let settings = state.settings.read().await;
    Ok(Json(settings.clone()))
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    Json(new_settings): Json<Settings>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Updating settings");

    let mut settings = state.settings.write().await;
    *settings = new_settings;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Settings updated successfully"
    })))
}

pub async fn get_applications(
    State(state): State<Arc<AppState>>,
    Path(ip): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut port = 22;
    {
        let hosts = state.hosts.read().await;
        if let Some(h) = hosts.iter().find(|h| h.ip == ip) {
            port = h.port;
        }
    }

    match state.discovery.get_applications_for_host(&ip, port).await {
        Ok(apps) => Ok(Json(apps)),
        Err(e) => {
            tracing::error!("Failed to get applications for host {}: {}", ip, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(serde::Deserialize)]
pub struct LaunchRequest {
    pub command: String,
}

pub async fn launch_application(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(payload): Json<LaunchRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Launching application in session {}: {}",
        session_id, payload.command
    );

    let session_info = {
        let sessions = state.sessions.read().await;
        sessions
            .iter()
            .find(|s| s.id == session_id)
            .map(|s| (s.host_ip.clone(), s.display_id))
    };

    if let Some((host_ip, display_id)) = session_info {
        let mut port = 22;
        {
            let hosts = state.hosts.read().await;
            if let Some(h) = hosts.iter().find(|h| h.ip == host_ip) {
                port = h.port;
            }
        }

        match state
            .discovery
            .launch_application_on_host(&host_ip, port, display_id, &payload.command)
            .await
        {
            Ok(res) => Ok(Json(res)),
            Err(e) => {
                tracing::error!(
                    "Failed to launch application in session {}: {}",
                    session_id,
                    e
                );
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        if let Some(stripped) = session_id.strip_prefix("system-display-") {
            if let Ok(display_id) = stripped.parse::<u32>() {
                let host_ip = "127.0.0.1".to_string();
                let port = 22;
                match state
                    .discovery
                    .launch_application_on_host(&host_ip, port, display_id, &payload.command)
                    .await
                {
                    Ok(res) => return Ok(Json(res)),
                    Err(e) => {
                        tracing::error!(
                            "Failed to launch application in physical session {}: {}",
                            session_id,
                            e
                        );
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
        }
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(serde::Deserialize)]
pub struct VncQueryParams {
    pub host: String,
    pub display: u32,
    #[serde(default)]
    pub monitor: u32,
    pub token: String,
}

pub async fn vnc_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<VncQueryParams>,
    State(state): State<Arc<AppState>>,
) -> Result<impl axum::response::IntoResponse, StatusCode> {
    let is_valid = {
        let tokens = state.active_tokens.read().await;
        tokens.contains(&params.token)
    };
    if !is_valid {
        tracing::warn!("Unauthorized VNC connection: invalid token");
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_vnc_socket(socket, params, state)))
}

async fn handle_vnc_socket(ws: WebSocket, params: VncQueryParams, state: Arc<AppState>) {
    info!(
        "Upgraded connection to WebSocket for VNC. Host: {}, Display: {}, Monitor: {}",
        params.host, params.display, params.monitor
    );

    let mut port = 2222;
    {
        let hosts = state.hosts.read().await;
        if let Some(h) = hosts.iter().find(|h| h.ip == params.host) {
            port = h.port;
        }
    }

    let vnc_port = match state
        .discovery
        .ensure_vnc_on_host(&params.host, port, params.display)
        .await
    {
        Ok(res) => {
            if let Some(err_msg) = res.get("error").and_then(|e| e.as_str()) {
                tracing::error!(
                    "Agent refused to start VNC for display {}: {}",
                    params.display,
                    err_msg
                );
                let mut ws_sender = ws.split().0;
                let _ = ws_sender
                    .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code: 4000,
                        reason: err_msg.to_string().into(),
                    })))
                    .await;
                return;
            }
            if let Some(p) = res.get("port").and_then(|p| p.as_u64()) {
                p as u16
            } else {
                tracing::warn!(
                    "Agent response missing port field, using default for display {}",
                    params.display
                );
                5900 + params.display as u16
            }
        }
        Err(e) => {
            tracing::error!("Failed to ensure VNC on host {}: {}", params.host, e);
            let mut ws_sender = ws.split().0;
            let _ = ws_sender
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 4001,
                    reason: format!("Agent communication error: {}", e).into(),
                })))
                .await;
            return;
        }
    };

    let target_addr = format!("{}:{}", params.host, vnc_port);
    info!(
        "Connecting WebSocket proxy to VNC TCP target: {}",
        target_addr
    );

    let tcp_stream = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        TcpStream::connect(&target_addr),
    )
    .await
    {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            tracing::error!("Failed to connect to VNC target {}: {}", target_addr, e);
            return;
        }
        Err(_) => {
            tracing::error!("Timeout connecting to VNC target {}", target_addr);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws.split();
    let (mut tcp_reader, mut tcp_writer) = tcp_stream.into_split();

    let client_to_server = async {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Binary(bin) => {
                    if tcp_writer.write_all(&bin).await.is_err() {
                        break;
                    }
                }
                Message::Text(txt) => {
                    if tcp_writer.write_all(txt.as_bytes()).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        let _ = tcp_writer.shutdown().await;
    };

    let server_to_client = async {
        let mut buf = [0u8; 16384];
        while let Ok(n) = tcp_reader.read(&mut buf).await {
            if n == 0 {
                break;
            }
            if ws_sender
                .send(Message::Binary(buf[..n].to_vec().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    };

    tokio::select! {
        _ = client_to_server => {}
        _ = server_to_client => {}
    }

    info!(
        "VNC WebSocket proxy connection closed for host={}",
        params.host
    );
}

pub async fn upload_file(
    axum::extract::Path(filename): axum::extract::Path<String>,
    body: axum::body::Bytes,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let desktop = std::path::PathBuf::from(home).join("Desktop");

    let _ = std::fs::create_dir_all(&desktop);

    let file_path = desktop.join(&filename);
    tracing::info!("Uploading file to: {:?}", file_path);

    match std::fs::write(&file_path, body) {
        Ok(_) => Ok(axum::Json(serde_json::json!({
            "success": true,
            "message": format!("Файл успешно загружен на рабочий стол: {}", filename)
        }))),
        Err(e) => {
            tracing::error!("Failed to save uploaded file: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_system_users(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(ip): axum::extract::Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let port = state.discovery.get_port_for_host(&ip);
    match state.discovery.get_system_users_for_host(&ip, port).await {
        Ok(json) => {
            if let Some(users) = json.get("users").and_then(|u| u.as_array()) {
                let list = users
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                Ok(Json(list))
            } else {
                Ok(Json(vec![]))
            }
        }
        Err(e) => {
            tracing::error!("Failed to get users for host {}: {}", ip, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(serde::Deserialize)]
pub struct PowerRequest {
    pub action: String,
}

pub async fn get_metrics(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(ip): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let port = state.discovery.get_port_for_host(&ip);
    match state.discovery.get_metrics_for_host(&ip, port).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => {
            tracing::error!("Failed to get metrics for host {}: {}", ip, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn execute_power_action(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(ip): axum::extract::Path<String>,
    Json(payload): Json<PowerRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let port = state.discovery.get_port_for_host(&ip);
    match state
        .discovery
        .execute_power_action_on_host(&ip, port, &payload.action)
        .await
    {
        Ok(res) => Ok(Json(res)),
        Err(e) => {
            tracing::error!(
                "Failed to execute power action {} for host {}: {}",
                payload.action,
                ip,
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
