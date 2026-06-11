use crate::events::InputEvent;
use crate::traits::{InputAuditSink, InputInjector, InputPolicy};
use anyhow::{bail, Result};
use std::sync::{Arc, Mutex};

pub struct MockInputInjector {
    policy: Arc<dyn InputPolicy>,
    audit: Arc<dyn InputAuditSink>,
    events: Arc<Mutex<Vec<InputEvent>>>,
}

impl MockInputInjector {
    pub fn new(policy: Arc<dyn InputPolicy>, audit: Arc<dyn InputAuditSink>) -> Self {
        Self {
            policy,
            audit,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn injected_events(&self) -> Vec<InputEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl InputInjector for MockInputInjector {
    fn inject(&self, event: InputEvent) -> Result<()> {
        if !self.policy.is_allowed(&event) {
            self.audit.audit_event(
                "INPUT_REJECTED",
                &format!("Policy blocked event: {:?}", event),
            );
            bail!("Input event rejected by security policy");
        }

        self.audit
            .audit_event("INPUT_INJECTED", &format!("Event: {:?}", event));
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}
