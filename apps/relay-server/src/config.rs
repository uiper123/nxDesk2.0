use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelayConfig {
    pub bind_address: String,
    pub port: u16,
    pub agent_token: String,
    pub client_token: String,
    pub heartbeat_timeout_secs: u64,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            agent_token: "agent_secret_pass".to_string(),
            client_token: "client_secret_pass".to_string(),
            heartbeat_timeout_secs: 5,
        }
    }
}
