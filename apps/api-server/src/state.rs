use crate::discovery::HostDiscovery;
use crate::models::{ActiveSession, Host, HostStatus, LogEntry, Settings};
use tokio::sync::RwLock;

pub struct AppState {
    pub hosts: RwLock<Vec<Host>>,
    pub sessions: RwLock<Vec<ActiveSession>>,
    pub logs: RwLock<Vec<LogEntry>>,
    pub settings: RwLock<Settings>,
    pub discovered_hosts: RwLock<Vec<Host>>,
    pub discovery: HostDiscovery,
}

impl AppState {
    pub async fn new() -> Self {
        let discovery = HostDiscovery::new();

        // Обнаружение реальных хостов при запуске
        let hosts = discovery.discover_hosts().await;

        Self {
            hosts: RwLock::new(hosts),
            sessions: RwLock::new(vec![]),
            logs: RwLock::new(vec![]),
            settings: RwLock::new(Settings {
                quality: "auto".to_string(),
                encoder: "vaapi".to_string(),
                fps: 30,
                audio: false,
            }),
            discovered_hosts: RwLock::new(vec![]),
            discovery,
        }
    }

    /// Обновить статус всех хостов
    pub async fn refresh_hosts(&self) {
        let mut hosts = self.hosts.write().await;
        self.discovery.refresh_host_status(&mut hosts).await;

        let mut all_sessions = Vec::new();
        let mut all_logs = Vec::new();

        for host in hosts.iter_mut() {
            if matches!(host.status, HostStatus::Online) {
                let port = host.port;
                match self
                    .discovery
                    .get_active_sessions_for_host(&host.ip, port)
                    .await
                {
                    Ok(mut host_sessions) => {
                        host.active_sessions = host_sessions.len() as u32;
                        all_sessions.append(&mut host_sessions);
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to get active sessions for host {}: {}",
                            host.ip,
                            e
                        );
                    }
                }

                match self.discovery.get_logs_for_host(&host.ip, port, 50).await {
                    Ok(mut host_logs) => {
                        all_logs.append(&mut host_logs);
                    }
                    Err(e) => {
                        tracing::error!("Failed to get logs for host {}: {}", host.ip, e);
                    }
                }
            }
        }

        // Sort logs by timestamp descending (newest first)
        all_logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_logs.truncate(100);

        let mut sessions = self.sessions.write().await;
        *sessions = all_sessions;

        let mut logs = self.logs.write().await;
        *logs = all_logs;
    }
}
