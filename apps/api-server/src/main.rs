mod discovery;
mod handlers;
mod models;
mod state;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting TTGTiSO-Desk API Server...");

    let state = Arc::new(AppState::new().await);

    // Клонируем Arc для фоновой задачи
    let state_clone = state.clone();
    let state_udp = state.clone();

    let app = Router::new()
        .route("/api/auth/login", post(handlers::login))
        .route("/api/sessions/start", post(handlers::start_session))
        .route("/api/hosts", get(handlers::get_hosts).post(handlers::add_host))
        .route("/api/hosts/discovered", get(handlers::get_discovered_hosts))
        .route("/api/sessions/active", get(handlers::get_active_sessions))
        .route(
            "/api/sessions/{id}/terminate",
            post(handlers::terminate_session),
        )
        .route(
            "/api/hosts/{ip}/applications",
            get(handlers::get_applications),
        )
        .route(
            "/api/hosts/{ip}/users",
            get(handlers::get_system_users),
        )
        .route(
            "/api/sessions/{id}/launch",
            post(handlers::launch_application),
        )
        .route("/api/upload/{filename}", post(handlers::upload_file))
        .route("/api/logs", get(handlers::get_logs))
        .route("/api/settings", get(handlers::get_settings))
        .route("/api/settings", post(handlers::update_settings))
        .route("/api/health", get(handlers::health_check))
        .route("/api/ws/vnc", get(handlers::vnc_ws_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    info!("API Server listening on http://127.0.0.1:3001");

    // Фоновая задача для периодического обновления статуса хостов
    tokio::spawn(async move {
        state_clone.refresh_hosts().await;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            info!("Refreshing host status...");
            state_clone.refresh_hosts().await;
        }
    });

    // Фоновая задача для автоматического обнаружения агентов по UDP
    tokio::spawn(async move {
        use crate::models::{Host, HostStatus};
        use tokio::net::UdpSocket;

        let socket = match UdpSocket::bind("0.0.0.0:9999").await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to bind UDP socket for auto-discovery: {}", e);
                return;
            }
        };
        info!("UDP Auto-Discovery listener bound to port 9999");
        let mut buf = [0u8; 1024];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, _addr)) => {
                    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&buf[..len]) {
                        if let (Some(name), Some(port)) = (
                            val.get("name").and_then(|v| v.as_str()),
                            val.get("port").and_then(|v| v.as_u64()),
                        ) {
                            let ip = _addr.ip().to_string();
                            let mut discovered = state_udp.discovered_hosts.write().await;
                            
                            if let Some(existing) = discovered.iter_mut().find(|h| h.ip == ip) {
                                let beacon_port = port as u16;
                                if existing.port != beacon_port {
                                    existing.port = beacon_port;
                                }
                                existing.status = HostStatus::Online;
                            } else {
                                info!("Auto-discovered new host: {} ({}:{})", name, ip, port);
                                let next_id = (discovered.len() + 1).to_string();

                                let os = if ip == "127.0.0.1" || ip == "localhost" {
                                    crate::discovery::HostDiscovery::detect_local_os()
                                } else {
                                    "Astra Linux".to_string()
                                };

                                discovered.push(Host {
                                    id: next_id,
                                    name: name.to_string(),
                                    ip: ip.clone(),
                                    port: port as u16,
                                    status: HostStatus::Online,
                                    active_sessions: 0,
                                    operating_system: os,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error receiving UDP discovery packet: {}", e);
                }
            }
        }
    });

    axum::serve(listener, app).await?;

    Ok(())
}
