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
        // 1. Initialize logging (ignore error if already set, e.g. service mode)
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);

        info!("Starting TTGTiSO-Desk Server Agent...");

        // 2. Load configuration from real file or defaults
        let config = config::load_config().unwrap_or_else(|e| {
            warn!("Config loading failed: {}. Using defaults.", e);
            config::AgentConfig::default()
        });

        info!(
            "Configuration loaded: bind={}:{}, max_sessions={}, audit_enabled={}",
            config.bind_address,
            config.port,
            config.session_limits.max_concurrent_sessions,
            config.security_policy.enable_audit_logs
        );

        // 3. Initialize components
        let audit_log_path = platform::default_audit_log_path();
        let audit_log = Arc::new(AuditLog::new(&audit_log_path));
        let session_mgr = Arc::new(LocalSessionManager::new_default());

        audit_log.log_auth_success("system", "127.0.0.1");
        info!("Audit log initialized at {:?}", audit_log_path);

        // 4. Create shutdown cancellation token
        let (shutdown_tx, _shutdown_rx) = broadcast::channel::<()>(1);

        // 5. Start local control listener (UDS on Linux, localhost TCP elsewhere)
        let control_task = tokio::spawn(crate::socket::run_control_listener(
            session_mgr.clone(),
            shutdown_tx.subscribe(),
        ));

        // 6. Start TCP connection listener with real protocol handler
        let port = config.port;
        let conn_task = tokio::spawn(crate::handler::run_connection_listener(
            port,
            session_mgr.clone(),
            audit_log.clone(),
            shutdown_tx.subscribe(),
        ));

        // 6.5. Spawn UDP broadcast beacon for auto-discovery
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

        // 7. Wait for termination signals
        info!(
            "Server Agent is fully initialized. Ready for connections on port {}.",
            port
        );

        wait_for_shutdown(external_shutdown.as_mut()).await?;

        // 8. Graceful shutdown sequence
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

/// Block until a shutdown condition is met: an OS termination signal, or a
/// message on the optional external shutdown channel (Windows service stop).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing_defaults() {
        let config = config::load_config().unwrap();
        assert_eq!(config.port, 22);
        assert!(config.security_policy.allow_password_auth);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_graceful_shutdown_mechanism() {
        let temp_dir = std::env::current_dir().unwrap().join("target");
        std::fs::create_dir_all(&temp_dir).ok();
        let uds_path = temp_dir.join("test_agent_shutdown.sock");

        let (shutdown_tx, _shutdown_rx) = broadcast::channel::<()>(1);
        let session_mgr = Arc::new(LocalSessionManager::new_default());

        let uds_path_str = uds_path.to_string_lossy().to_string();
        let uds_task = tokio::spawn(crate::socket::run_uds_listener(
            uds_path_str,
            session_mgr,
            shutdown_tx.subscribe(),
        ));

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let send_res = shutdown_tx.send(());
        assert!(send_res.is_ok());

        let task_res = tokio::time::timeout(tokio::time::Duration::from_secs(2), uds_task).await;

        assert!(
            task_res.is_ok(),
            "UDS listener did not shut down gracefully in time"
        );
    }
}
