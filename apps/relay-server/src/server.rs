use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader};
use tokio::sync::mpsc;
use crate::config::RelayConfig;
use crate::session::SessionRegistry;
use audit::{AuditLog, AuditRecord};

#[derive(Serialize, Deserialize, Debug)]
struct HandshakeRequest {
    role: String,
    token: String,
    session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct HandshakeResponse {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

pub struct RelayServer {
    pub config: RelayConfig,
    pub registry: SessionRegistry,
    pub audit: Arc<AuditLog>,
}

impl RelayServer {
    pub fn new(config: RelayConfig, registry: SessionRegistry, audit: Arc<AuditLog>) -> Self {
        Self { config, registry, audit }
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let bind_addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let listener = TcpListener::bind(&bind_addr).await?;
        tracing::info!("Relay server listening on {}", bind_addr);

        // Start heartbeat watchdog
        let self_clone = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let expired = self_clone.registry.check_heartbeats(self_clone.config.heartbeat_timeout_secs);
                for id in expired {
                    tracing::warn!("Session {} expired due to heartbeat timeout", id);
                    self_clone.audit.write_record(AuditRecord {
                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        event_type: "RELAY_SESSION".to_string(),
                        username: "relay".to_string(),
                        ip_address: "127.0.0.1".to_string(),
                        action: "heartbeat_timeout".to_string(),
                        details: format!("Terminated session {} due to lack of heartbeat", id),
                    });
                }
            }
        });

        loop {
            let (socket, _addr) = listener.accept().await?;
            let self_ref = self.clone();
            tokio::spawn(async move {
                if let Err(e) = self_ref.handle_connection(socket).await {
                    tracing::error!("Connection handler failed: {:?}", e);
                }
            });
        }
    }

    async fn handle_connection(&self, socket: tokio::net::TcpStream) -> Result<()> {
        let (reader, mut writer) = socket.into_split();
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        // 1. Handshake authentication phase
        buf_reader.read_line(&mut line).await?;
        let req: HandshakeRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => {
                let err_resp = serde_json::to_vec(&HandshakeResponse {
                    status: "error".to_string(),
                    message: Some("Invalid handshake".to_string()),
                })?;
                writer.write_all(&err_resp).await?;
                writer.write_all(b"\n").await?;
                bail!("Handshake deserialization failed");
            }
        };

        // Validate credentials without logging secrets
        let expected_token = if req.role == "agent" {
            &self.config.agent_token
        } else if req.role == "client" {
            &self.config.client_token
        } else {
            bail!("Unknown role in handshake");
        };

        if req.token != *expected_token {
            let err_resp = serde_json::to_vec(&HandshakeResponse {
                status: "error".to_string(),
                message: Some("Unauthorized".to_string()),
            })?;
            writer.write_all(&err_resp).await?;
            writer.write_all(b"\n").await?;
            self.audit.write_record(AuditRecord {
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                event_type: "RELAY_AUTH".to_string(),
                username: "unknown".to_string(),
                ip_address: "127.0.0.1".to_string(),
                action: "handshake_failed".to_string(),
                details: format!("Handshake token mismatch for role: {}", req.role),
            });
            bail!("Unauthorized handshake token");
        }

        // Send success response
        let ok_resp = serde_json::to_vec(&HandshakeResponse {
            status: "ok".to_string(),
            message: None,
        })?;
        writer.write_all(&ok_resp).await?;
        writer.write_all(b"\n").await?;

        // 2. Routing phase
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let role = req.role.clone();
        let session_id = req.session_id.clone();

        if role == "agent" {
            self.registry.register_agent(&session_id, tx);
        } else {
            self.registry.register_client(&session_id, tx);
        }

        self.audit.write_record(AuditRecord {
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            event_type: "RELAY_SESSION".to_string(),
            username: "relay".to_string(),
            ip_address: "127.0.0.1".to_string(),
            action: format!("register_{}", role),
            details: format!("Registered {} connection for session {}", role, session_id),
        });

        // Loop writing data sent from peer to this connection
        let mut writer_task = tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                if writer.write_all(&data).await.is_err() {
                    break;
                }
            }
        });

        // Loop reading from this connection and routing to peer
        let registry = self.registry.clone();
        let session_id_clone = session_id.clone();
        let role_clone = role.clone();
        
        let mut reader_task = tokio::spawn(async move {
            let mut buffer = vec![0; 4096];
            loop {
                // For agent, reading also serves as heartbeat presence update
                if role_clone == "agent" {
                    registry.update_heartbeat(&session_id_clone);
                }

                match buf_reader.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let payload = buffer[..n].to_vec();
                        if role_clone == "agent" {
                            registry.route_to_client(&session_id_clone, payload);
                        } else {
                            registry.route_to_agent(&session_id_clone, payload);
                        }
                    }
                }
            }
        });

        // Wait until connection closes
        tokio::select! {
            _ = &mut writer_task => {},
            _ = &mut reader_task => {},
        };

        // Clean up
        self.registry.unregister(&session_id);
        self.audit.write_record(AuditRecord {
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            event_type: "RELAY_SESSION".to_string(),
            username: "relay".to_string(),
            ip_address: "127.0.0.1".to_string(),
            action: format!("unregister_{}", role),
            details: format!("Unregistered {} connection for session {}", role, session_id),
        });

        Ok(())
    }
}
