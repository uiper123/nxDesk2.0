use crate::discovery::HostDiscovery;
use crate::models::{ActiveSession, Host, HostStatus, LogEntry, Settings};
use tokio::sync::RwLock;

pub struct AppState {
    pub hosts: RwLock<Vec<Host>>,
    pub sessions: RwLock<Vec<ActiveSession>>,
    pub logs: RwLock<Vec<LogEntry>>,
    pub settings: RwLock<Settings>,
    pub discovered_hosts: RwLock<Vec<Host>>,
    pub active_tokens: RwLock<std::collections::HashSet<String>>,
    pub discovery: HostDiscovery,
}

impl AppState {
    pub async fn new() -> Self {
        let discovery = HostDiscovery::new();
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
            active_tokens: RwLock::new(std::collections::HashSet::new()),
            discovery,
        }
    }

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
                        host.status = if host_sessions.is_empty() {
                            HostStatus::Online
                        } else {
                            HostStatus::Busy
                        };
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

        all_logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_logs.truncate(100);

        let mut sessions = self.sessions.write().await;
        *sessions = all_sessions;

        let mut logs = self.logs.write().await;
        *logs = all_logs;
    }

    pub async fn resolve_host_port(
        &self,
        host_input: &str,
        requested_port: Option<u16>,
    ) -> (String, u16) {
        let target = self
            .discovery
            .normalize_remote_target(host_input, requested_port, None);
        if requested_port.filter(|p| *p != 0).is_some() {
            return (target.host, target.port);
        }

        let hosts = self.hosts.read().await;
        let port = hosts
            .iter()
            .find(|h| {
                let (_, h_host) = HostDiscovery::parse_ssh_target(&h.ip);
                h_host == target.host
            })
            .map(|h| h.port)
            .filter(|p| *p != 0)
            .unwrap_or_else(|| {
                if target.port != 0 {
                    target.port
                } else {
                    self.discovery.get_port_for_host(host_input)
                }
            });

        (target.host, port)
    }
}
