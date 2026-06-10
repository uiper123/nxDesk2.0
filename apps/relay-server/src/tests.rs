#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tokio::net::TcpStream;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader};
    use tokio::time::{sleep, Duration};
    use crate::config::RelayConfig;
    use crate::session::SessionRegistry;
    use crate::server::RelayServer;
    use audit::AuditLog;

    async fn start_test_server() -> (u16, Arc<RelayServer>) {
        let config = RelayConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 0, // OS assigns a free ephemeral port
            agent_token: "agent_secret".to_string(),
            client_token: "client_secret".to_string(),
            heartbeat_timeout_secs: 1, // small timeout for testing
        };
        let registry = SessionRegistry::new();
        let temp_dir = std::env::temp_dir();
        let audit = Arc::new(AuditLog::new(&temp_dir.join("test_relay_server.log")));
        
        let server = Arc::new(RelayServer::new(config, registry, audit));
        
        // Retrieve bound port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener); // free the port and let server bind it in loop

        // Start server in background
        let mut server_config = server.config.clone();
        server_config.port = port;
        let server_with_port = Arc::new(RelayServer::new(server_config, server.registry.clone(), server.audit.clone()));
        
        let server_clone = server_with_port.clone();
        tokio::spawn(async move {
            let _ = server_clone.run().await;
        });

        // wait for server startup
        sleep(Duration::from_millis(150)).await;

        (port, server_with_port)
    }

    #[tokio::test]
    async fn test_unauthorized_connection_rejection() {
        let (port, _) = start_test_server().await;
        
        let mut socket = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
        
        // Send invalid token handshake
        let handshake = r#"{"role":"client","token":"wrong_secret","session_id":"session-1"}"#;
        socket.write_all(handshake.as_bytes()).await.unwrap();
        socket.write_all(b"\n").await.unwrap();

        let mut reader = BufReader::new(socket);
        let mut resp = String::new();
        reader.read_line(&mut resp).await.unwrap();

        assert!(resp.contains("error"));
        assert!(resp.contains("Unauthorized"));
    }

    #[tokio::test]
    async fn test_client_agent_relay_connection() {
        let (port, _) = start_test_server().await;

        // 1. Connect agent
        let mut agent_socket = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
        let agent_handshake = r#"{"role":"agent","token":"agent_secret","session_id":"session-test-1"}"#;
        agent_socket.write_all(agent_handshake.as_bytes()).await.unwrap();
        agent_socket.write_all(b"\n").await.unwrap();

        let mut agent_reader = BufReader::new(agent_socket);
        let mut agent_resp = String::new();
        agent_reader.read_line(&mut agent_resp).await.unwrap();
        assert!(agent_resp.contains("ok"));

        // 2. Connect client
        let mut client_socket = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
        let client_handshake = r#"{"role":"client","token":"client_secret","session_id":"session-test-1"}"#;
        client_socket.write_all(client_handshake.as_bytes()).await.unwrap();
        client_socket.write_all(b"\n").await.unwrap();

        let mut client_reader = BufReader::new(client_socket);
        let mut client_resp = String::new();
        client_reader.read_line(&mut client_resp).await.unwrap();
        assert!(client_resp.contains("ok"));

        // 3. Forward client data to agent
        let test_payload = b"secure_frame_payload_data";
        client_reader.get_mut().write_all(test_payload).await.unwrap();

        let mut incoming_agent_payload = vec![0; test_payload.len()];
        agent_reader.read_exact(&mut incoming_agent_payload).await.unwrap();
        assert_eq!(incoming_agent_payload, test_payload);
    }

    #[tokio::test]
    async fn test_heartbeat_timeout() {
        let (port, server) = start_test_server().await;

        // Connect agent
        let mut agent_socket = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
        let agent_handshake = r#"{"role":"agent","token":"agent_secret","session_id":"session-heartbeat-1"}"#;
        agent_socket.write_all(agent_handshake.as_bytes()).await.unwrap();
        agent_socket.write_all(b"\n").await.unwrap();

        let mut agent_reader = BufReader::new(agent_socket);
        let mut agent_resp = String::new();
        agent_reader.read_line(&mut agent_resp).await.unwrap();
        assert!(agent_resp.contains("ok"));

        assert!(server.registry.is_agent_present("session-heartbeat-1"));

        // Sleep to exceed the 1 second heartbeat timeout (no activity sent)
        sleep(Duration::from_millis(2000)).await;

        // Agent session should be garbage collected/expired by server watchdog
        assert!(!server.registry.is_agent_present("session-heartbeat-1"));
    }

    #[tokio::test]
    async fn test_reconnect_behavior() {
        let (port, server) = start_test_server().await;

        // Connect agent first time
        {
            let mut agent_socket = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
            let agent_handshake = r#"{"role":"agent","token":"agent_secret","session_id":"session-reconnect-1"}"#;
            agent_socket.write_all(agent_handshake.as_bytes()).await.unwrap();
            agent_socket.write_all(b"\n").await.unwrap();
            let mut agent_reader = BufReader::new(agent_socket);
            let mut agent_resp = String::new();
            agent_reader.read_line(&mut agent_resp).await.unwrap();
            assert!(agent_resp.contains("ok"));
            assert!(server.registry.is_agent_present("session-reconnect-1"));
            // agent_socket closes here when it goes out of scope
        }

        // Wait a bit for server cleanup
        sleep(Duration::from_millis(150)).await;
        assert!(!server.registry.is_agent_present("session-reconnect-1"));

        // Connect agent second time (reconnect)
        let mut agent_socket_2 = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
        let agent_handshake_2 = r#"{"role":"agent","token":"agent_secret","session_id":"session-reconnect-1"}"#;
        agent_socket_2.write_all(agent_handshake_2.as_bytes()).await.unwrap();
        agent_socket_2.write_all(b"\n").await.unwrap();

        let mut agent_reader_2 = BufReader::new(agent_socket_2);
        let mut agent_resp_2 = String::new();
        agent_reader_2.read_line(&mut agent_resp_2).await.unwrap();
        assert!(agent_resp_2.contains("ok"));

        assert!(server.registry.is_agent_present("session-reconnect-1"));
    }
}
