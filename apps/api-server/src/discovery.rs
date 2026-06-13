use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTarget {
    pub host: String,
    pub username: Option<String>,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SshCommandSpec {
    host: String,
    username: Option<String>,
    port: u16,
    key_path: Option<String>,
    batch_mode: bool,
    strict_host_key_checking: bool,
}

#[derive(Debug, Clone, Copy)]
enum SshOperation {
    GetStatus,
    GetSessions,
    GetLogs,
    StopSession,
    StartSession,
    GetApplications,
    GetUsers,
    GetMetrics,
    Power,
    LaunchApplication,
    EnsureVnc,
}

impl SshOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::GetStatus => "get_status",
            Self::GetSessions => "get_sessions",
            Self::GetLogs => "get_logs",
            Self::StopSession => "stop_session",
            Self::StartSession => "start_session",
            Self::GetApplications => "get_applications",
            Self::GetUsers => "get_users",
            Self::GetMetrics => "get_metrics",
            Self::Power => "power",
            Self::LaunchApplication => "launch_application",
            Self::EnsureVnc => "ensure_vnc",
        }
    }
}

impl HostDiscovery {
    const LOCAL_AGENT_SOCKET: &'static str = "/var/lib/ttgtiso-desk/agent.sock";
    const DEFAULT_SSH_PORT: u16 = 22;

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

    #[cfg(unix)]
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

    #[cfg(not(unix))]
    async fn run_local_agent_command(&self, _command: &str) -> Result<String, anyhow::Error> {
        Err(anyhow::anyhow!(
            "Unix sockets are not supported on this platform"
        ))
    }

    pub fn parse_ssh_target(target: &str) -> (Option<&str>, &str) {
        if let Some(pos) = target.find('@') {
            let user = &target[..pos];
            let host = &target[pos + 1..];
            (
                if user.is_empty() { None } else { Some(user) },
                if host.is_empty() { target } else { host },
            )
        } else {
            (None, target)
        }
    }

    pub fn normalize_remote_target(
        &self,
        host_input: &str,
        port: Option<u16>,
        explicit_username: Option<&str>,
    ) -> RemoteTarget {
        let (embedded_username, parsed_host) = Self::parse_ssh_target(host_input.trim());
        let host = parsed_host.trim().to_string();
        let config = self.find_config_for_host(&host);
        let config_username = config
            .and_then(|c| Self::parse_ssh_target(&c.ip).0)
            .map(str::to_string);
        let username = explicit_username
            .filter(|u| !u.trim().is_empty())
            .map(|u| u.trim().to_string())
            .or_else(|| embedded_username.map(str::to_string))
            .or(config_username);

        if let (Some(embedded), Some(explicit)) = (embedded_username, explicit_username) {
            if !explicit.trim().is_empty() && embedded != explicit.trim() {
                warn!(
                    "Remote target contains username '{}', but explicit username '{}' was requested; using explicit username",
                    embedded,
                    explicit.trim()
                );
            }
        }

        let resolved_port = port
            .filter(|p| *p != 0)
            .or_else(|| config.map(|c| c.ssh_port).filter(|p| *p != 0))
            .unwrap_or(Self::DEFAULT_SSH_PORT);

        RemoteTarget {
            host,
            username,
            port: resolved_port,
        }
    }

    /// Проверка доступности хоста: локально через UDS агента, удаленно по TCP порту
    async fn check_host_availability(&self, ip: &str, port: u16) -> bool {
        let target = self.normalize_remote_target(ip, Some(port), None);
        if Self::is_local_host(&target.host) {
            return self.check_local_agent_health().await;
        }

        let addr = format!("{}:{}", target.host, target.port);

        match timeout(Self::discovery_timeout(), TcpStream::connect(&addr)).await {
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
        operation: SshOperation,
        duration: Duration,
    ) -> Result<std::process::Output, anyhow::Error> {
        match timeout(duration, cmd.output()).await {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(e)) => Err(anyhow::anyhow!("Command execution failed: {}", e)),
            Err(_) => Err(anyhow::anyhow!(
                "SSH command timed out after {} seconds while running operation {}.",
                duration.as_secs(),
                operation.as_str()
            )),
        }
    }

    fn discovery_timeout() -> Duration {
        std::env::var("TTGTISO_DISCOVERY_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(2))
    }

    fn ssh_timeout() -> Duration {
        std::env::var("TTGTISO_SSH_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(10))
    }

    /// Получение статуса агента (активные сессии и т.д.) через UDS по SSH или локально
    async fn get_agent_status(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let target = self.normalize_remote_target(ip, Some(port), None);
        if Self::is_local_host(&target.host) {
            let json_str = self.run_local_agent_command("status").await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        }

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = self.create_ssh_command(SshOperation::GetStatus, &target);
        cmd.arg(format!(
            "echo 'status' | socat - UNIX-CONNECT:{} || echo 'status' | nc -U {}",
            sock_path, sock_path
        ));

        let output = self
            .run_command_with_timeout(cmd, SshOperation::GetStatus, Self::ssh_timeout())
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "{}",
                Self::format_ssh_error(SshOperation::GetStatus, &target, &output.stderr)
            )
        }
    }

    /// Получение списка активных сессий через UDS
    pub async fn get_active_sessions_for_host(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<Vec<crate::models::ActiveSession>, anyhow::Error> {
        let target = self.normalize_remote_target(ip, Some(port), None);
        let json_str = if Self::is_local_host(&target.host) {
            self.run_local_agent_command("sessions").await?
        } else {
            let sock_path = Self::LOCAL_AGENT_SOCKET;
            let mut c = self.create_ssh_command(SshOperation::GetSessions, &target);
            c.arg(format!(
                "echo 'sessions' | socat - UNIX-CONNECT:{} || echo 'sessions' | nc -U {}",
                sock_path, sock_path
            ));
            let output = self
                .run_command_with_timeout(c, SshOperation::GetSessions, Self::ssh_timeout())
                .await?;

            if !output.status.success() {
                anyhow::bail!(
                    "{}",
                    Self::format_ssh_error(SshOperation::GetSessions, &target, &output.stderr)
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
                    host_ip: target.host.clone(),
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
        let target = self.normalize_remote_target(ip, Some(port), None);
        let is_online = self
            .check_host_availability(&target.host, target.port)
            .await;
        if !is_online {
            anyhow::bail!(
                "Host {}:{} is offline or unreachable.",
                target.host,
                target.port
            );
        }

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let cmd_payload = format!("stop_session {}", session_id);

        if Self::is_local_host(&target.host) {
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

        let mut cmd = self.create_ssh_command(SshOperation::StopSession, &target);
        cmd.arg(format!(
            "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
            cmd_payload, sock_path, cmd_payload, sock_path
        ));

        let output = self
            .run_command_with_timeout(cmd, SshOperation::StopSession, Self::ssh_timeout())
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
                "{}",
                Self::format_ssh_error(SshOperation::StopSession, &target, &output.stderr)
            )
        }
    }

    /// Получить порт для указанного хоста из конфигурации
    pub fn get_port_for_host(&self, ip: &str) -> u16 {
        let (_, target_ip) = Self::parse_ssh_target(ip);
        self.config_hosts
            .iter()
            .find(|c| {
                let (_, config_ip) = Self::parse_ssh_target(&c.ip);
                config_ip == target_ip
            })
            .map(|c| c.ssh_port)
            .unwrap_or(Self::DEFAULT_SSH_PORT)
    }

    fn find_config_for_host(&self, ip: &str) -> Option<&HostConfig> {
        let (_, target_ip) = Self::parse_ssh_target(ip);
        self.config_hosts.iter().find(|c| {
            let (_, config_ip) = Self::parse_ssh_target(&c.ip);
            config_ip == target_ip
        })
    }

    fn get_ssh_key_path(&self, ip: &str) -> Option<String> {
        let mut key_path = None;
        let (_, target_ip) = Self::parse_ssh_target(ip);
        if let Ok(content) = std::fs::read_to_string("hosts.toml") {
            if let Ok(config) = toml::from_str::<HostsConfig>(&content) {
                for c in config.hosts {
                    let (_, config_ip) = Self::parse_ssh_target(&c.ip);
                    if config_ip == target_ip {
                        key_path = c.ssh_private_key_path;
                        break;
                    }
                }
            }
        }

        if key_path.is_none() {
            #[cfg(target_os = "windows")]
            let default_path = std::env::var("USERPROFILE").ok().map(|h| {
                format!(
                    "{}\\AppData\\Local\\TTGTiSO\\TTGTiSO-Desk\\ssh\\id_ed25519",
                    h
                )
            });
            #[cfg(not(target_os = "windows"))]
            let default_path = std::env::var("HOME")
                .ok()
                .map(|h| format!("{}/.local/share/ttgtiso-desk/ssh/id_ed25519", h));

            if let Some(path) = default_path {
                if std::path::Path::new(&path).exists() {
                    key_path = Some(path);
                }
            }
        }

        key_path
    }

    fn build_ssh_command_spec(&self, target: &RemoteTarget) -> SshCommandSpec {
        SshCommandSpec {
            host: target.host.clone(),
            username: target.username.clone(),
            port: target.port,
            key_path: self.get_ssh_key_path(&target.host),
            batch_mode: true,
            strict_host_key_checking: false,
        }
    }

    fn ssh_target_arg(spec: &SshCommandSpec) -> String {
        match &spec.username {
            Some(username) if !username.is_empty() => format!("{}@{}", username, spec.host),
            _ => spec.host.clone(),
        }
    }

    fn command_template(spec: &SshCommandSpec) -> String {
        let target = Self::ssh_target_arg(spec);
        let key = if spec.key_path.is_some() {
            " -i <key>"
        } else {
            ""
        };
        format!(
            "ssh -o StrictHostKeyChecking={} -o BatchMode={} -p {}{} {} <remote-command>",
            if spec.strict_host_key_checking {
                "yes"
            } else {
                "no"
            },
            if spec.batch_mode { "yes" } else { "no" },
            spec.port,
            key,
            target
        )
    }

    fn auth_method(spec: &SshCommandSpec) -> &'static str {
        if spec.key_path.is_some() {
            "key"
        } else if spec.batch_mode {
            "key/agent"
        } else {
            "unknown"
        }
    }

    fn create_ssh_command(&self, operation: SshOperation, target: &RemoteTarget) -> Command {
        let spec = self.build_ssh_command_spec(target);
        info!(
            operation = operation.as_str(),
            host = %spec.host,
            port = spec.port,
            username = spec.username.as_deref().unwrap_or("<none>"),
            auth_method = Self::auth_method(&spec),
            timeout_secs = Self::ssh_timeout().as_secs(),
            command_template = %Self::command_template(&spec),
            "Preparing SSH command"
        );

        let mut cmd = Command::new("ssh");
        cmd.arg("-o").arg(format!(
            "StrictHostKeyChecking={}",
            if spec.strict_host_key_checking {
                "yes"
            } else {
                "no"
            }
        ));
        cmd.arg("-o").arg(format!(
            "BatchMode={}",
            if spec.batch_mode { "yes" } else { "no" }
        ));
        cmd.arg("-p").arg(spec.port.to_string());

        if let Some(path) = &spec.key_path {
            cmd.arg("-i").arg(path);
        }

        cmd.arg(Self::ssh_target_arg(&spec));
        cmd
    }

    fn format_ssh_error(operation: SshOperation, target: &RemoteTarget, stderr: &[u8]) -> String {
        let stderr = String::from_utf8_lossy(stderr);
        let stderr = stderr.trim();
        let username = target.username.as_deref().unwrap_or("<default ssh user>");

        if stderr.contains("Permission denied") {
            format!(
                "SSH authentication failed for user {} on host {}:{}. Check username, password/key, sshd config. {}",
                username, target.host, target.port, stderr
            )
        } else if stderr.contains("No route to host") {
            format!(
                "Host is reachable on checked port {}, but SSH command used port {}. Check port configuration. {}",
                target.port, target.port, stderr
            )
        } else {
            format!(
                "SSH command failed while running operation {} for user {} on host {}:{}. {}",
                operation.as_str(),
                username,
                target.host,
                target.port,
                stderr
            )
        }
    }

    /// Запуск сессии на указанном хосте
    pub async fn start_session_on_host(
        &self,
        ip: &str,
        port: u16,
        username: &str,
    ) -> Result<crate::models::ActiveSession, anyhow::Error> {
        use anyhow::Context;
        let target = self.normalize_remote_target(ip, Some(port), Some(username));
        let is_online = self
            .check_host_availability(&target.host, target.port)
            .await;
        if !is_online {
            anyhow::bail!(
                "Host {}:{} is offline or unreachable.",
                target.host,
                target.port
            );
        }

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let cmd_payload = format!("start_session {}", username);

        if Self::is_local_host(&target.host) {
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
                    host_ip: target.host.clone(),
                });
            }

            let err = json_val
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            anyhow::bail!("{}", err);
        };

        let mut cmd = self.create_ssh_command(SshOperation::StartSession, &target);
        cmd.arg(format!(
            "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
            cmd_payload, sock_path, cmd_payload, sock_path
        ));

        let output = self
            .run_command_with_timeout(cmd, SshOperation::StartSession, Self::ssh_timeout())
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
                    host_ip: target.host.clone(),
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
                "{}",
                Self::format_ssh_error(SshOperation::StartSession, &target, &output.stderr)
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
        let target = self.normalize_remote_target(ip, Some(port), None);
        let cmd = if Self::is_local_host(&target.host) {
            let mut c = Command::new("sh");
            c.arg("-c").arg(format!("tail -n {} {}", lines, log_path));
            c
        } else {
            let mut c = self.create_ssh_command(SshOperation::GetLogs, &target);
            c.arg(format!("tail -n {} {}", lines, log_path));
            c
        };

        let output = self
            .run_command_with_timeout(cmd, SshOperation::GetLogs, Self::ssh_timeout())
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
                        message: format!("[{}] {}: {}", target.host, username, details),
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
                    let (_, actual_ip) = Self::parse_ssh_target(&config.ip);
                    if Self::is_local_host(actual_ip) {
                        os = Self::detect_local_os();
                    } else {
                        os = "Linux".to_string();
                    }
                }
            }

            let target = self.normalize_remote_target(&config.ip, Some(config.ssh_port), None);
            hosts.push(Host {
                id: id_counter.to_string(),
                name: config.name.clone(),
                ip: target.host,
                port: target.port,
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
            let target = self.normalize_remote_target(&host.ip, Some(host.port), None);
            host.ip = target.host.clone();
            host.port = target.port;
            let is_online = self
                .check_host_availability(&target.host, target.port)
                .await;

            host.status = if is_online {
                HostStatus::Online
            } else {
                HostStatus::Offline
            };

            let mut os = "Unknown / Offline".to_string();
            if is_online {
                if let Ok(agent_status) = self.get_agent_status(&target.host, target.port).await {
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
                    let (_, actual_ip) = Self::parse_ssh_target(&host.ip);
                    if Self::is_local_host(actual_ip) {
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
        let target = self.normalize_remote_target(ip, Some(port), None);
        if Self::is_local_host(&target.host) {
            let json_str = self.run_local_agent_command("applications").await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = self.create_ssh_command(SshOperation::GetApplications, &target);
        cmd.arg(format!(
            "echo 'applications' | socat - UNIX-CONNECT:{} || echo 'applications' | nc -U {}",
            sock_path, sock_path
        ));

        let output = self
            .run_command_with_timeout(cmd, SshOperation::GetApplications, Self::ssh_timeout())
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "{}",
                Self::format_ssh_error(SshOperation::GetApplications, &target, &output.stderr)
            )
        }
    }

    pub async fn get_system_users_for_host(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let target = self.normalize_remote_target(ip, Some(port), None);
        if Self::is_local_host(&target.host) {
            let json_str = self.run_local_agent_command("users").await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = self.create_ssh_command(SshOperation::GetUsers, &target);
        cmd.arg(format!(
            "echo 'users' | socat - UNIX-CONNECT:{} || echo 'users' | nc -U {}",
            sock_path, sock_path
        ));

        let output = self
            .run_command_with_timeout(cmd, SshOperation::GetUsers, Self::ssh_timeout())
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "{}",
                Self::format_ssh_error(SshOperation::GetUsers, &target, &output.stderr)
            )
        }
    }

    pub async fn get_metrics_for_host(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let target = self.normalize_remote_target(ip, Some(port), None);
        if Self::is_local_host(&target.host) {
            let json_str = self.run_local_agent_command("metrics").await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = self.create_ssh_command(SshOperation::GetMetrics, &target);
        cmd.arg(format!(
            "echo 'metrics' | socat - UNIX-CONNECT:{} || echo 'metrics' | nc -U {}",
            sock_path, sock_path
        ));

        let output = self
            .run_command_with_timeout(cmd, SshOperation::GetMetrics, Self::ssh_timeout())
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "{}",
                Self::format_ssh_error(SshOperation::GetMetrics, &target, &output.stderr)
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
        let target = self.normalize_remote_target(ip, Some(port), None);
        if Self::is_local_host(&target.host) {
            let json_str = self.run_local_agent_command(&cmd_payload).await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let sock_path = Self::LOCAL_AGENT_SOCKET;
        let mut cmd = self.create_ssh_command(SshOperation::Power, &target);
        cmd.arg(format!(
            "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
            cmd_payload, sock_path, cmd_payload, sock_path
        ));

        let output = self
            .run_command_with_timeout(cmd, SshOperation::Power, Self::ssh_timeout())
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "{}",
                Self::format_ssh_error(SshOperation::Power, &target, &output.stderr)
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

        let target = self.normalize_remote_target(ip, Some(port), None);
        if Self::is_local_host(&target.host) {
            let json_str = self.run_local_agent_command(&cmd_payload).await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let mut cmd = self.create_ssh_command(SshOperation::LaunchApplication, &target);
        cmd.arg(format!(
            "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
            cmd_payload, sock_path, cmd_payload, sock_path
        ));

        let output = self
            .run_command_with_timeout(cmd, SshOperation::LaunchApplication, Self::ssh_timeout())
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "{}",
                Self::format_ssh_error(SshOperation::LaunchApplication, &target, &output.stderr)
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

        let target = self.normalize_remote_target(ip, Some(port), None);
        if Self::is_local_host(&target.host) {
            let json_str = self.run_local_agent_command(&cmd_payload).await?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            return Ok(json_val);
        };

        let mut cmd = self.create_ssh_command(SshOperation::EnsureVnc, &target);
        cmd.arg(format!(
            "echo '{}' | socat - UNIX-CONNECT:{} || echo '{}' | nc -U {}",
            cmd_payload, sock_path, cmd_payload, sock_path
        ));

        let output = self
            .run_command_with_timeout(cmd, SshOperation::EnsureVnc, Self::ssh_timeout())
            .await?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json_val: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(json_val)
        } else {
            anyhow::bail!(
                "{}",
                Self::format_ssh_error(SshOperation::EnsureVnc, &target, &output.stderr)
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct HostsConfig {
    hosts: Vec<HostConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn discovery() -> HostDiscovery {
        HostDiscovery {
            config_hosts: vec![HostConfig {
                name: "remote".to_string(),
                ip: "usertest@192.168.1.46".to_string(),
                ssh_port: 22,
                ssh_public_key: None,
                ssh_public_key_path: None,
                ssh_private_key_path: None,
            }],
        }
    }

    #[test]
    fn explicit_user_and_port_are_used_in_ssh_template() {
        let discovery = discovery();
        let target = discovery.normalize_remote_target("192.168.1.46", Some(22), Some("vladimir"));
        let spec = discovery.build_ssh_command_spec(&target);
        let template = HostDiscovery::command_template(&spec);

        assert_eq!(target.host, "192.168.1.46");
        assert_eq!(target.username.as_deref(), Some("vladimir"));
        assert_eq!(target.port, 22);
        assert!(template.contains("-p 22"));
        assert!(template.contains("vladimir@192.168.1.46"));
    }

    #[test]
    fn embedded_user_is_parsed_when_explicit_user_is_missing() {
        let discovery = discovery();
        let target = discovery.normalize_remote_target("usertest@192.168.1.46", None, None);

        assert_eq!(target.host, "192.168.1.46");
        assert_eq!(target.username.as_deref(), Some("usertest"));
        assert_eq!(target.port, 22);
    }

    #[test]
    fn explicit_user_wins_over_embedded_user() {
        let discovery = discovery();
        let target =
            discovery.normalize_remote_target("usertest@192.168.1.46", None, Some("vladimir"));

        assert_eq!(target.host, "192.168.1.46");
        assert_eq!(target.username.as_deref(), Some("vladimir"));
    }

    #[test]
    fn requested_port_is_used_for_discovery_and_ssh_command() {
        let discovery = discovery();
        let target = discovery.normalize_remote_target("192.168.1.46", Some(2200), None);
        let spec = discovery.build_ssh_command_spec(&target);

        assert_eq!(target.port, 2200);
        assert!(HostDiscovery::command_template(&spec).contains("-p 2200"));
    }

    #[test]
    fn switching_users_does_not_reuse_previous_user() {
        let discovery = discovery();
        let testuser =
            discovery.normalize_remote_target("192.168.1.46", Some(22), Some("testuser"));
        let usertest =
            discovery.normalize_remote_target("192.168.1.46", Some(22), Some("usertest"));
        let vladimir =
            discovery.normalize_remote_target("192.168.1.46", Some(22), Some("vladimir"));

        assert_eq!(testuser.username.as_deref(), Some("testuser"));
        assert_eq!(usertest.username.as_deref(), Some("usertest"));
        assert_eq!(vladimir.username.as_deref(), Some("vladimir"));
    }
}
