use anyhow::Result;
use crate::events::InputEvent;

pub trait KeyboardMapper: Send + Sync {
    fn map_key(&self, keycode: u32, layout: &str) -> Result<u32>;
}

pub trait MouseMapper: Send + Sync {
    fn map_button(&self, button: shared_types::MouseButton) -> Result<u8>;
}

pub trait InputPolicy: Send + Sync {
    fn is_allowed(&self, event: &InputEvent) -> bool;
}

pub trait InputAuditSink: Send + Sync {
    fn audit_event(&self, event_type: &str, details: &str);
}

pub trait InputInjector: Send + Sync {
    fn inject(&self, event: InputEvent) -> Result<()>;
}
