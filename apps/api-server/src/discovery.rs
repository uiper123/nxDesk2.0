use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

use crate::models::{Host, HostStatus};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostConfig {
    pub name: String,
    pub ip: String,
    pub ssh_port: u16,
    pub ssh_public_key: Option<String>,
    pub ssh_public_key_path: Option<String>,
    pub ssh_private_key_path: Option<String>,
}

/// Сканер для обнаружения доступных хостов в сети
pub struct HostDiscovery {
    config_hosts: Vec<HostConfig>,
}

impl HostDiscovery {
    const LOCAL_AGENT_SOCKET: &'static str = "/var/lib/ttgtiso-desk/agent.sock";

    pub fn new() -> Self {
        Self {
            config_hosts: Self::load_config_hosts(),
        }
    }

    /// Загрузка хостов из конфигурационного файла
    fn load_config_hosts() -> Vec<HostConfig> {
        if let Ok(content) = std::fs::read_to_string("hosts.toml") {
            if let Ok(config) = toml::from_str::<HostsConfig>(&content) {
                return config.hosts;
            }
        }

        vec![HostConfig {
            name: "localhost-test".to_string(),
            ip: "127.0.0.1".to_string(),
            ssh_port: 22,
            ssh_public_key: None,
            ssh_public_key_path: None,
            ssh_private_key_path: None,
        }]
    }

    pub fn detect_local_os() -> String {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    let name = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                    return name.to_string();
                }
            }
        }
        "Astra Linux".to_string()
    }

    pub fn is_local_host(ip: &str) -> bool {
        if ip == "127.0.0.1" || ip == "localhost" || ip == "::1" {
            return true;
        }

        if let Ok(ip_addr) = ip.parse::<std::net::IpAddr>() {
            if ip_addr.is_loopback() {
                return true;
            }
            // Попытка привязать UDP-сокет к этому IP-адресу.
            // Если IP-адрес принадлежит локальному интерфейсу, привязка (bind) завершится успешно.
            // Если IP-адрес внешний/удаленный, вернется ошибка AddrNotAvailable.
            match std::net::UdpSocket::bind((ip_addr, 0)) {
                Ok(_) => true,
                Err(e) => e.kind() != std::io::ErrorKind::AddrNotAvailable,
            }
        } else {
            false
        }
    }

    async fn check_local_agent_health(&self) -> bool {
        match self.get_agent_status("127.0.0.1", 0).await {
            Ok(status) if status.get("status").and_then(|v| v.as_str()) == Some("OK") => {
                info!("Local server-agent is reachable via Unix socket");
                true
            }
            Ok(_) => {
                warn!("Local server-agent Unix socket returned an unexpected health payload");
                false
            }
            Err(e) => {
                warn!("Local server-agent Unix socket is unreachable: {}", e);
                false
            }
        }
    }

    async fn run_local_agent_command(&self, command: &str) -> Result<String, anyhow::Error> {
        let mut stream = timeout(
            Duration::from_secs(2),
            UnixStream::connect(Self::LOCAL_AGENT_SOCKET),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Timed out connecting to {}", Self::LOCAL_AGENT_SOCKET))??;

        stream.write_all(command.as_bytes()).await?;
        stream.shutdown().await?;

        let mut response = String::new();
        timeout(Duration::from_secs(4), stream.read_to_string(&mut response))
            .await
            .map_err(|_| {
                anyhow::anyhow!("Timed out reading from {}", Self::LOCAL_AGENT_SOCKET)
            })??;

        Ok(response)
    }

    /// Проверка доступности хоста: локально через UDS агента, удаленно по TCP порту
    async fn check_host_availability(&self, ip: &str, port: u16) -> bool {
        if Self::is_local_host(ip) {
            return self.check_local_agent_health().await;
        }

        let addr = format!("{}:{}", ip, port);

        match timeout(Duration::from_secs(2), TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => {
                info!("Host {} is reachable", addr);
                true
            }
            Ok(Err(_)) | Err(_) => {
                warn!("Host {} is unreachable", addr);
                false
            }
        }
    }

    /// Выполнение команды с таймаутом
    async fn run_command_with_timeout(
        &self,
        mut cmd: Command,
        duration: Duration,
    ) -> Result<std::process::Output, anyhow::Error> {
        match timeout(duration, cmd.output()).await {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(e)) => Err(anyhow::anyhow!("Command execution failed: {}", e)),
            Err(_) => Err(anyhow::anyhow!("Command timed out after {:?}", duration)),
        }
    }

    /// Получение статуса агента (активные сессии и т.д.) через UDS по SSH или локально
    async fn get_agent_status(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<serde_json::Value, anyhow::Error> {
        if Self::is_local_host(ip) {
            let json_str = self.run_local_agent_command("status").await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        }

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            "-p",
            &port.to_string(),
            ip,
            &format!(
                "echo 'status' | socat - UNIX-CONNECT:{} || echo 'status' | nc -U {}",
                sock_path, sock_path
            ),
        ]);

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    /// Получение списка активных сессий через UDS
    pub async fn get_active_sessions_for_host(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<Vec<crate::models::ActiveSession>, anyhow::Error> {
        let json_str = if Self::is_local_host(ip) {
            self.run_local_agent_command("sessions").await?
        } else {
            let sock_path = Self::LOCAL_AGENT_SOCKET;
            let mut c = Command::new("ssh");
            c.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "BatchMode=yes",
                "-p",
                &port.to_string(),
                ip,
                &format!(
                    "echo 'sessions' | socat - UNIX-CONNECT:{} || echo 'sessions' | nc -U {}",
                    sock_path, sock_path
                ),
            ]);
            let output = self
                .run_command_with_timeout(c, Duration::from_secs(4))
                .await?;

            if !output.status.success() {
                anyhow::bail!(
                    "Command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
            }

            String::from_utf8_lossy(&output.stdout).into_owned()
        };

        let json_val: serde_json::Value = serde_json::from_str(&json_str)?;

        let mut result = Vec::new();
        if let Some(sessions) = json_val.get("sessions").and_then(|s| s.as_array()) {
            for s in sessions {
                result.push(crate::models::ActiveSession {
                    id: s
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    username: s
                        .get("username")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    display_id: s.get("display_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    start_time: s
                        .get("duration_seconds")
                        .map(|v| format!("{}s ago", v.as_u64().unwrap_or(0)))
                        .unwrap_or_else(|| "unknown".to_string()),
                    cpu_usage: 0.0, // Can be pulled from process metrics later
                    mem_usage: 0,
                    host_ip: ip.to_string(),
                });
            }
        }
        Ok(result)
    }

    /// Остановка сессии на указанном хосте
    pub async fn stop_session_on_host(
        &self,
        ip: &str,
        port: u16,
        session_id: &str,
    ) -> Result<(), anyhow::Error> {
        let is_online = self.check_host_availability(ip, port).await;
        if !is_online {
            anyhow::bail!("Host {} is offline or unreachable.", ip);
        }

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let cmd_payload = format!("stop_session {}", session_id);

        if Self::is_local_host(ip) {
            let json_str = self.run_local_agent_command(&cmd_payload).await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            if json_val
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return Ok(());
            }

            let err = json_val
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            anyhow::bail!("{}", err);
        };

        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            "-p",
            &port.to_string(),
            ip,
            &format!(
                "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
                cmd_payload, sock_path, cmd_payload, sock_path
            ),
        ]);

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            if json_val
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                Ok(())
            } else {
                let err = json_val
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                anyhow::bail!("{}", err)
            }
        } else {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    /// Получить порт для указанного хоста из конфигурации
    pub fn get_port_for_host(&self, ip: &str) -> u16 {
        self.config_hosts
            .iter()
            .find(|c| c.ip == ip)
            .map(|c| c.ssh_port)
            .unwrap_or(22)
    }

    /// Запуск сессии на указанном хосте
    pub async fn start_session_on_host(
        &self,
        ip: &str,
        port: u16,
        username: &str,
    ) -> Result<crate::models::ActiveSession, anyhow::Error> {
        use anyhow::Context;
        let is_online = self.check_host_availability(ip, port).await;
        if !is_online {
            anyhow::bail!("Host {} is offline or unreachable.", ip);
        }

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let cmd_payload = format!("start_session {}", username);

        if Self::is_local_host(ip) {
            let json_str = self.run_local_agent_command(&cmd_payload).await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            if json_val
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                let s = json_val
                    .get("session")
                    .context("Missing session in response")?;
                return Ok(crate::models::ActiveSession {
                    id: s
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    username: s
                        .get("username")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    display_id: s.get("display_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    start_time: "just now".to_string(),
                    cpu_usage: 0.0,
                    mem_usage: 0,
                    host_ip: ip.to_string(),
                });
            }

            let err = json_val
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            anyhow::bail!("{}", err);
        };

        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            "-p",
            &port.to_string(),
            ip,
            &format!(
                "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
                cmd_payload, sock_path, cmd_payload, sock_path
            ),
        ]);

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            if json_val
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                let s = json_val
                    .get("session")
                    .context("Missing session in response")?;
                Ok(crate::models::ActiveSession {
                    id: s
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    username: s
                        .get("username")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    display_id: s.get("display_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    start_time: "just now".to_string(),
                    cpu_usage: 0.0,
                    mem_usage: 0,
                    host_ip: ip.to_string(),
                })
            } else {
                let err = json_val
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                anyhow::bail!("{}", err)
            }
        } else {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    /// Получение последних логов агента
    pub async fn get_logs_for_host(
        &self,
        ip: &str,
        port: u16,
        lines: usize,
    ) -> Result<Vec<crate::models::LogEntry>, anyhow::Error> {
        let log_path = "/var/log/ttgtiso-desk/audit.log";
        let cmd = if Self::is_local_host(ip) {
            let mut c = Command::new("sh");
            c.arg("-c").arg(format!("tail -n {} {}", lines, log_path));
            c
        } else {
            let mut c = Command::new("ssh");
            c.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "BatchMode=yes",
                "-p",
                &port.to_string(),
                ip,
                &format!("tail -n {} {}", lines, log_path),
            ]);
            c
        };

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let mut result = Vec::new();
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    let timestamp = val.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);
                    let event_type = val
                        .get("event_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN");
                    let details = val.get("details").and_then(|v| v.as_str()).unwrap_or("");
                    let username = val.get("username").and_then(|v| v.as_str()).unwrap_or("");

                    let level = match event_type {
                        "AUTHENTICATION" => crate::models::LogLevel::Audit,
                        "ERROR" => crate::models::LogLevel::Error,
                        "WARN" => crate::models::LogLevel::Warn,
                        _ => crate::models::LogLevel::Info,
                    };

                    // Simple formatting of UNIX timestamp
                    let ts_str = format!("{}", timestamp);

                    result.push(crate::models::LogEntry {
                        timestamp: ts_str,
                        level,
                        message: format!("[{}] {}: {}", ip, username, details),
                    });
                }
            }
            Ok(result)
        } else {
            // Ignore if file doesn't exist yet
            Ok(Vec::new())
        }
    }

    /// Сканирование всех настроенных хостов
    pub async fn discover_hosts(&self) -> Vec<Host> {
        let mut hosts = Vec::new();

        for (id_counter, config) in (1..).zip(self.config_hosts.iter()) {
            let is_online = self
                .check_host_availability(&config.ip, config.ssh_port)
                .await;

            let mut active_sessions = 0;
            let status = if is_online {
                HostStatus::Online
            } else {
                HostStatus::Offline
            };

            let mut os = "Unknown / Offline".to_string();
            let mut agent_res = None;

            if is_online {
                match timeout(
                    Duration::from_secs(3),
                    self.get_agent_status(&config.ip, config.ssh_port),
                )
                .await
                {
                    Ok(Ok(agent_status)) => {
                        if let Some(sessions) =
                            agent_status.get("active_sessions").and_then(|s| s.as_u64())
                        {
                            active_sessions = sessions as u32;
                        }
                        agent_res = Some(agent_status);
                    }
                    Ok(Err(e)) => {
                        warn!("Agent on {} is unreachable via UDS: {}", config.ip, e);
                    }
                    Err(_) => {
                        warn!("Timeout getting agent status from {}", config.ip);
                    }
                }

                if let Some(ref status) = agent_res {
                    if let Some(sys) = status.get("system") {
                        if let Some(os_name) = sys.get("os").and_then(|o| o.as_str()) {
                            os = if os_name == "windows" {
                                "Windows".to_string()
                            } else if os_name == "linux" {
                                "Linux".to_string()
                            } else {
                                os_name.to_string()
                            };
                        }
                    }
                }
                if os == "Unknown / Offline" || os.is_empty() {
                    if Self::is_local_host(&config.ip) {
                        os = Self::detect_local_os();
                    } else {
                        os = "Linux".to_string();
                    }
                }
            }

            hosts.push(Host {
                id: id_counter.to_string(),
                name: config.name.clone(),
                ip: config.ip.clone(),
                port: config.ssh_port,
                status,
                active_sessions,
                operating_system: os,
                ssh_public_key: config.ssh_public_key.clone(),
                ssh_public_key_path: config.ssh_public_key_path.clone(),
                ssh_private_key_path: config.ssh_private_key_path.clone(),
            });
        }

        info!("Discovered {} hosts", hosts.len());
        hosts
    }

    /// Периодическое обновление статуса хостов
    pub async fn refresh_host_status(&self, hosts: &mut [Host]) {
        for host in hosts.iter_mut() {
            let port = self.get_port_for_host(&host.ip);
            let is_online = self.check_host_availability(&host.ip, port).await;

            host.status = if is_online {
                HostStatus::Online
            } else {
                HostStatus::Offline
            };

            let mut os = "Unknown / Offline".to_string();
            if is_online {
                if let Ok(agent_status) = self.get_agent_status(&host.ip, port).await {
                    if let Some(_sessions) =
                        agent_status.get("active_sessions").and_then(|s| s.as_u64())
                    {
                        host.active_sessions = 0; // Let refresh_hosts query active_sessions directly
                    }
                    if let Some(sys) = agent_status.get("system") {
                        if let Some(os_name) = sys.get("os").and_then(|o| o.as_str()) {
                            os = if os_name == "windows" {
                                "Windows".to_string()
                            } else if os_name == "linux" {
                                "Linux".to_string()
                            } else {
                                os_name.to_string()
                            };
                        }
                    }
                }
                if os == "Unknown / Offline" || os.is_empty() {
                    if Self::is_local_host(&host.ip) {
                        os = Self::detect_local_os();
                    } else {
                        os = "Linux".to_string();
                    }
                }
            } else {
                host.active_sessions = 0;
            }
            host.operating_system = os;
        }
    }

    pub async fn get_applications_for_host(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<serde_json::Value, anyhow::Error> {
        if Self::is_local_host(ip) {
            let json_str = self.run_local_agent_command("applications").await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            "-p",
            &port.to_string(),
            ip,
            &format!(
                "echo 'applications' | socat - UNIX-CONNECT:{} || echo 'applications' | nc -U {}",
                sock_path, sock_path
            ),
        ]);

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    pub async fn get_system_users_for_host(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<serde_json::Value, anyhow::Error> {
        if Self::is_local_host(ip) {
            let json_str = self.run_local_agent_command("users").await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            "-p",
            &port.to_string(),
            ip,
            &format!(
                "echo 'users' | socat - UNIX-CONNECT:{} || echo 'users' | nc -U {}",
                sock_path, sock_path
            ),
        ]);

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    pub async fn get_metrics_for_host(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<serde_json::Value, anyhow::Error> {
        if Self::is_local_host(ip) {
            let json_str = self.run_local_agent_command("metrics").await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            "-p",
            &port.to_string(),
            ip,
            &format!(
                "echo 'metrics' | socat - UNIX-CONNECT:{} || echo 'metrics' | nc -U {}",
                sock_path, sock_path
            ),
        ]);

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    pub async fn execute_power_action_on_host(
        &self,
        ip: &str,
        port: u16,
        action: &str,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let cmd_payload = format!("power {}", action);
        if Self::is_local_host(ip) {
            let json_str = self.run_local_agent_command(&cmd_payload).await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            "-p",
            &port.to_string(),
            ip,
            &format!(
                "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
                cmd_payload, sock_path, cmd_payload, sock_path
            ),
        ]);

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    pub async fn launch_application_on_host(
        &self,
        ip: &str,
        port: u16,
        display_id: u32,
        command: &str,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let cmd_payload = format!("launch {} {}", display_id, command);

        if Self::is_local_host(ip) {
            let json_str = self.run_local_agent_command(&cmd_payload).await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            "-p",
            &port.to_string(),
            ip,
            &format!(
                "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
                cmd_payload, sock_path, cmd_payload, sock_path
            ),
        ]);

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    pub async fn ensure_vnc_on_host(
        &self,
        ip: &str,
        port: u16,
        display_id: u32,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let cmd_payload = format!("ensure_vnc {}", display_id);

        if Self::is_local_host(ip) {
            let json_str = self.run_local_agent_command(&cmd_payload).await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            "-p",
            &port.to_string(),
            ip,
            &format!(
                "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
                cmd_payload, sock_path, cmd_payload, sock_path
            ),
        ]);

        let output = self
            .run_command_with_timeout(cmd, Duration::from_secs(4))
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct HostsConfig {
    hosts: Vec<HostConfig>,
}
