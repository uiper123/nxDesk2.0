use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use audit::AuditLog;
use session_manager::LocalSessionManager;

use crate::platform;

pub struct AgentApp;

impl AgentApp {
    pub async fn run() -> Result<()> {
        Self::run_with_shutdown(None).await
    }

    /// Run the agent. If `external_shutdown` is provided, the agent will also
    /// shut down when a value is received on it (used by the Windows service
    /// control handler). Otherwise it waits for OS termination signals.
    pub async fn run_with_shutdown(
        mut external_shutdown: Option<broadcast::Receiver<()>>,
    ) -> Result<()> {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);

        info!("Starting TTGTiSO-Desk Server Agent...");

        let config = config::load_config().unwrap_or_else(|e| {
            warn!("Config loading failed: {}. Using defaults.", e);
            config::AgentConfig::default()
        });

        info!(
            "Configuration loaded: bind={}:{}, mode={:?}, desktop_session={:?}, max_sessions={}, audit_enabled={}",
            config.bind_address,
            config.port,
            config.connection_mode,
            config.desktop_session_type,
            config.session_limits.max_concurrent_sessions,
            config.security_policy.enable_audit_logs
        );

        let audit_log_path = platform::default_audit_log_path();
        let audit_log = Arc::new(AuditLog::new(&audit_log_path));
        let session_mgr = Arc::new(LocalSessionManager::from_config(&config));

        audit_log.log_auth_success("system", "127.0.0.1");
        info!("Audit log initialized at {:?}", audit_log_path);

        let (shutdown_tx, _shutdown_rx) = broadcast::channel::<()>(1);

        let control_task = tokio::spawn(crate::socket::run_control_listener(
            session_mgr.clone(),
            shutdown_tx.subscribe(),
        ));

        let port = config.port;
        let conn_task = tokio::spawn(crate::handler::run_connection_listener(
            config.clone(),
            session_mgr.clone(),
            audit_log.clone(),
            shutdown_tx.subscribe(),
        ));

        let agent_name = platform::agent_hostname();
        tokio::spawn(async move {
            use tokio::net::UdpSocket;
            let socket = match UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => {
                    let _ = s.set_broadcast(true);
                    s
                }
                Err(e) => {
                    warn!(
                        "Failed to bind UDP socket for auto-discovery broadcast: {}",
                        e
                    );
                    return;
                }
            };
            let target_addr_broadcast = "255.255.255.255:9999";
            let target_addr_local = "127.0.0.1:9999";
            info!("UDP auto-discovery broadcast active");
            loop {
                let payload = serde_json::json!({
                    "name": agent_name,
                    "port": port,
                });
                if let Ok(data) = serde_json::to_vec(&payload) {
                    let _ = socket.send_to(&data, target_addr_broadcast).await;
                    let _ = socket.send_to(&data, target_addr_local).await;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        info!(
            "Server Agent is fully initialized. Ready for connections on port {}.",
            port
        );

        wait_for_shutdown(external_shutdown.as_mut()).await?;

        info!("Graceful shutdown initiated. Notifying background tasks...");

        let stats = crate::handler::connection_stats();
        info!("Final connection statistics: {}", stats);

        let _ = shutdown_tx.send(());

        tokio::select! {
            _ = async {
                let _ = control_task.await;
                let _ = conn_task.await;
            } => {
                info!("Background tasks stopped successfully.");
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                warn!("Graceful shutdown timed out. Force exiting.");
            }
        }

        info!("TTGTiSO-Desk Server Agent stopped.");
        Ok(())
    }
}

async fn wait_for_shutdown(external: Option<&mut broadcast::Receiver<()>>) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;

        match external {
            Some(rx) => {
                tokio::select! {
                    _ = sigterm.recv() => info!("Received SIGTERM signal"),
                    _ = sigint.recv() => info!("Received SIGINT signal"),
                    _ = rx.recv() => info!("Received external shutdown request"),
                }
            }
            None => {
                tokio::select! {
                    _ = sigterm.recv() => info!("Received SIGTERM signal"),
                    _ = sigint.recv() => info!("Received SIGINT signal"),
                }
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        match external {
            Some(rx) => {
                tokio::select! {
                    res = tokio::signal::ctrl_c() => {
                        res?;
                        info!("Received Ctrl+C / Stop signal");
                    }
                    _ = rx.recv() => info!("Received external shutdown request"),
                }
            }
            None => {
                tokio::signal::ctrl_c().await?;
                info!("Received Ctrl+C / Stop signal");
            }
        }
    }
    Ok(())
}
