pub mod config;
pub mod session;
pub mod server;
pub mod tests;

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use crate::config::RelayConfig;
use crate::session::SessionRegistry;
use crate::server::RelayServer;
use audit::AuditLog;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting TTGTiSO-Desk Relay Server...");

    // Setup configuration
    let config = RelayConfig::default();
    let registry = SessionRegistry::new();
    
    // Setup audit log path
    let temp_dir = std::env::temp_dir();
    let log_file = temp_dir.join("relay_audit.log");
    let audit = Arc::new(AuditLog::new(&log_file));

    let server = Arc::new(RelayServer::new(config, registry, audit));

    let server_task = tokio::spawn(server.run());

    info!("Relay Server is running. Press Ctrl+C to exit.");
    
    tokio::select! {
        res = server_task => {
            if let Err(e) = res {
                tracing::error!("Server crashed: {:?}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl+C received, stopping Relay Server...");
        }
    }

    Ok(())
}
