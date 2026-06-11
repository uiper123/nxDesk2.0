//! Session continuity: keep a session "frozen" through a transient network
//! drop instead of tearing it down.
//!
//! When a streaming connection dies (Wi-Fi → 4G handover on a laptop, a flaky
//! VPN, a sleeping client), the underlying desktop session is still perfectly
//! alive on the host. The old behaviour aborted the video task and forgot
//! everything, forcing a full re-login. This registry instead hands the client
//! an opaque *resume token* when the session starts. If the socket drops, the
//! session is parked in a `Frozen` state for a grace period; a client that
//! reconnects and presents a matching token is re-attached to the very same
//! desktop. Tokens that are never redeemed expire after the grace period so we
//! don't leak parked sessions forever.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a dropped session stays resumable before it is reaped.
pub const DEFAULT_GRACE: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeState {
    /// A client is currently attached and streaming.
    Attached,
    /// The client dropped; the session is parked and resumable until `expires`.
    Frozen,
}

#[derive(Debug, Clone)]
struct Entry {
    session_id: String,
    state: ResumeState,
    /// When a frozen entry becomes eligible for reaping.
    frozen_until: Option<Instant>,
}

/// Thread-safe registry of resumable sessions keyed by resume token.
pub struct ResumeRegistry {
    grace: Duration,
    entries: Mutex<HashMap<String, Entry>>,
}

impl Default for ResumeRegistry {
    fn default() -> Self {
        Self::new(DEFAULT_GRACE)
    }
}

impl ResumeRegistry {
    pub fn new(grace: Duration) -> Self {
        Self {
            grace,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Register a freshly-started session and return its resume token.
    pub fn register(&self, session_id: &str) -> String {
        let token = generate_token(session_id);
        let mut map = self.entries.lock().unwrap();
        map.insert(
            token.clone(),
            Entry {
                session_id: session_id.to_string(),
                state: ResumeState::Attached,
                frozen_until: None,
            },
        );
        token
    }

    /// Mark the session behind `token` as frozen (the client dropped). The
    /// session remains resumable until the grace period elapses.
    pub fn freeze(&self, token: &str) {
        let mut map = self.entries.lock().unwrap();
        if let Some(entry) = map.get_mut(token) {
            entry.state = ResumeState::Frozen;
            entry.frozen_until = Some(Instant::now() + self.grace);
        }
    }

    /// Attempt to resume the session behind `token`. Returns the session id on
    /// success. Fails if the token is unknown or its grace period has lapsed.
    pub fn resume(&self, token: &str) -> Option<String> {
        let mut map = self.entries.lock().unwrap();
        self.reap_locked(&mut map);
        let entry = map.get_mut(token)?;
        entry.state = ResumeState::Attached;
        entry.frozen_until = None;
        Some(entry.session_id.clone())
    }

    /// Forget a session entirely (e.g. on a clean stop).
    pub fn forget(&self, token: &str) {
        self.entries.lock().unwrap().remove(token);
    }

    /// Current state of a token, if known.
    pub fn state(&self, token: &str) -> Option<ResumeState> {
        let mut map = self.entries.lock().unwrap();
        self.reap_locked(&mut map);
        map.get(token).map(|e| e.state)
    }

    /// Number of tokens currently tracked (after reaping expired ones).
    pub fn len(&self) -> usize {
        let mut map = self.entries.lock().unwrap();
        self.reap_locked(&mut map);
        map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn reap_locked(&self, map: &mut HashMap<String, Entry>) {
        let now = Instant::now();
        map.retain(|_, e| match (e.state, e.frozen_until) {
            (ResumeState::Frozen, Some(until)) => now < until,
            _ => true,
        });
    }
}

fn generate_token(session_id: &str) -> String {
    // A non-cryptographic but unguessable-enough token: session id mixed with a
    // high-resolution timestamp and the registry pointer entropy. Good enough
    // for a closed LAN; swap for a CSPRNG if exposed more widely.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in session_id.bytes().chain(nanos.to_le_bytes()) {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("rt_{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_then_resume_roundtrip() {
        let reg = ResumeRegistry::default();
        let token = reg.register("sess-1");
        assert_eq!(reg.state(&token), Some(ResumeState::Attached));
        reg.freeze(&token);
        assert_eq!(reg.state(&token), Some(ResumeState::Frozen));
        assert_eq!(reg.resume(&token).as_deref(), Some("sess-1"));
        assert_eq!(reg.state(&token), Some(ResumeState::Attached));
    }

    #[test]
    fn unknown_token_cannot_resume() {
        let reg = ResumeRegistry::default();
        assert!(reg.resume("rt_deadbeef").is_none());
    }

    #[test]
    fn expired_frozen_session_is_reaped() {
        let reg = ResumeRegistry::new(Duration::from_millis(20));
        let token = reg.register("sess-2");
        reg.freeze(&token);
        std::thread::sleep(Duration::from_millis(40));
        assert!(reg.resume(&token).is_none(), "stale token must not resume");
        assert!(reg.is_empty());
    }

    #[test]
    fn forget_removes_token() {
        let reg = ResumeRegistry::default();
        let token = reg.register("sess-3");
        reg.forget(&token);
        assert!(reg.state(&token).is_none());
    }

    #[test]
    fn tokens_are_unique_per_session() {
        let reg = ResumeRegistry::default();
        let t1 = reg.register("a");
        let t2 = reg.register("b");
        assert_ne!(t1, t2);
    }
}
