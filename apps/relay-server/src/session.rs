use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Default)]
pub struct SessionRegistryInner {
    pub agents: HashMap<String, UnboundedSender<Vec<u8>>>,
    pub clients: HashMap<String, UnboundedSender<Vec<u8>>>,
    pub heartbeats: HashMap<String, std::time::Instant>,
}

#[derive(Clone, Default)]
pub struct SessionRegistry {
    inner: Arc<Mutex<SessionRegistryInner>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionRegistryInner::default())),
        }
    }

    pub fn register_agent(&self, session_id: &str, sender: UnboundedSender<Vec<u8>>) {
        let mut inner = self.inner.lock().unwrap();
        inner.agents.insert(session_id.to_string(), sender);
        inner.heartbeats.insert(session_id.to_string(), std::time::Instant::now());
    }

    pub fn register_client(&self, session_id: &str, sender: UnboundedSender<Vec<u8>>) {
        let mut inner = self.inner.lock().unwrap();
        inner.clients.insert(session_id.to_string(), sender);
    }

    pub fn unregister(&self, session_id: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.agents.remove(session_id);
        inner.clients.remove(session_id);
        inner.heartbeats.remove(session_id);
    }

    pub fn update_heartbeat(&self, session_id: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.heartbeats.insert(session_id.to_string(), std::time::Instant::now());
    }

    pub fn check_heartbeats(&self, timeout_secs: u64) -> Vec<String> {
        let mut inner = self.inner.lock().unwrap();
        let mut expired = Vec::new();
        let now = std::time::Instant::now();
        
        for (session_id, last_seen) in &inner.heartbeats {
            if now.duration_since(*last_seen).as_secs() >= timeout_secs {
                expired.push(session_id.clone());
            }
        }
        
        for id in &expired {
            inner.agents.remove(id);
            inner.clients.remove(id);
            inner.heartbeats.remove(id);
        }
        expired
    }

    pub fn route_to_client(&self, session_id: &str, data: Vec<u8>) -> bool {
        let inner = self.inner.lock().unwrap();
        if let Some(sender) = inner.clients.get(session_id) {
            sender.send(data).is_ok()
        } else {
            false
        }
    }

    pub fn route_to_agent(&self, session_id: &str, data: Vec<u8>) -> bool {
        let inner = self.inner.lock().unwrap();
        if let Some(sender) = inner.agents.get(session_id) {
            sender.send(data).is_ok()
        } else {
            false
        }
    }

    pub fn is_agent_present(&self, session_id: &str) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.agents.contains_key(session_id)
    }
}
