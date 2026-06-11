pub mod backends;
pub mod events;
pub mod policy;
pub mod traits;

pub use backends::mock::MockInputInjector;
pub use backends::x11::X11InputInjector;
pub use events::InputEvent;
pub use policy::SecureInputPolicy;
pub use traits::{InputAuditSink, InputInjector, InputPolicy, KeyboardMapper, MouseMapper};

use std::sync::Mutex;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::protocol::xtest::ConnectionExt as _;

// Direct X11InputInjector wrapper compatibility struct for other crates
pub struct LegacyX11InputInjector {
    _display: String,
}

impl LegacyX11InputInjector {
    pub fn new(display: &str) -> Self {
        Self {
            _display: display.to_string(),
        }
    }

    pub fn inject_mouse_move(&self, x: u16, y: u16) -> anyhow::Result<()> {
        let (conn, _) = x11rb::connect(Some(&self._display))?;
        conn.xtest_fake_input(
            6, // MotionNotify
            0,
            0,
            x11rb::NONE,
            x as i16,
            y as i16,
            0,
        )?;
        conn.flush()?;
        Ok(())
    }

    pub fn inject_mouse_click(
        &self,
        button: shared_types::MouseButton,
        pressed: bool,
    ) -> anyhow::Result<()> {
        let (conn, _) = x11rb::connect(Some(&self._display))?;
        let detail = match button {
            shared_types::MouseButton::Left => 1,
            shared_types::MouseButton::Middle => 2,
            shared_types::MouseButton::Right => 3,
            shared_types::MouseButton::None => return Ok(()),
        };
        let type_id = if pressed { 4 } else { 5 }; // ButtonPress = 4, ButtonRelease = 5
        conn.xtest_fake_input(type_id, detail, 0, x11rb::NONE, 0, 0, 0)?;
        conn.flush()?;
        Ok(())
    }

    pub fn inject_mouse_scroll(&self, delta: i16) -> anyhow::Result<()> {
        let (conn, _) = x11rb::connect(Some(&self._display))?;
        let detail = if delta > 0 { 4 } else { 5 };
        conn.xtest_fake_input(
            4, // ButtonPress
            detail,
            0,
            x11rb::NONE,
            0,
            0,
            0,
        )?;
        conn.xtest_fake_input(
            5, // ButtonRelease
            detail,
            0,
            x11rb::NONE,
            0,
            0,
            0,
        )?;
        conn.flush()?;
        Ok(())
    }

    pub fn inject_keypress(&self, keysym: u32, pressed: bool) -> anyhow::Result<()> {
        let (conn, _) = x11rb::connect(Some(&self._display))?;
        let setup = conn.setup();
        let min_keycode = setup.min_keycode;
        let max_keycode = setup.max_keycode;
        let count = max_keycode - min_keycode + 1;
        let reply = conn.get_keyboard_mapping(min_keycode, count)?.reply()?;

        let keysyms_per_keycode = reply.keysyms_per_keycode as usize;
        let mut target_keycode = None;
        for (i, sym) in reply.keysyms.iter().enumerate() {
            if *sym == keysym {
                let keycode = min_keycode as usize + (i / keysyms_per_keycode);
                target_keycode = Some(keycode as u8);
                break;
            }
        }

        if let Some(keycode) = target_keycode {
            let type_id = if pressed { 2 } else { 3 }; // KeyPress = 2, KeyRelease = 3
            conn.xtest_fake_input(type_id, keycode, 0, x11rb::NONE, 0, 0, 0)?;
            conn.flush()?;
        }
        Ok(())
    }
}

// Simple Audit implementation for tests
#[derive(Default)]
pub struct TestAuditSink {
    records: Mutex<Vec<(String, String)>>,
}

impl TestAuditSink {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    pub fn records(&self) -> Vec<(String, String)> {
        self.records.lock().unwrap().clone()
    }
}

impl InputAuditSink for TestAuditSink {
    fn audit_event(&self, event_type: &str, details: &str) {
        self.records
            .lock()
            .unwrap()
            .push((event_type.to_string(), details.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use traits::InputInjector;

    #[test]
    fn test_forbidden_hotkey_block() {
        let policy = Arc::new(SecureInputPolicy::new());
        let audit = Arc::new(TestAuditSink::new());
        let injector = MockInputInjector::new(policy, audit.clone());

        // Allowed combo
        let ev1 = InputEvent::Hotkey {
            combo: "Ctrl+C".to_string(),
        };
        assert!(injector.inject(ev1).is_ok());

        // Forbidden VT-switching combo
        let ev2 = InputEvent::Hotkey {
            combo: "Ctrl+Alt+F1".to_string(),
        };
        assert!(injector.inject(ev2).is_err());

        // Forbidden Ctrl+Alt+Delete combo
        let ev3 = InputEvent::Hotkey {
            combo: "Ctrl+Alt+Del".to_string(),
        };
        assert!(injector.inject(ev3).is_err());

        // Check audit log
        let records = audit.records();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].0, "INPUT_INJECTED");
        assert_eq!(records[1].0, "INPUT_REJECTED");
        assert_eq!(records[2].0, "INPUT_REJECTED");
    }

    #[test]
    fn test_mouse_event_injection() {
        let policy = Arc::new(SecureInputPolicy::new());
        let audit = Arc::new(TestAuditSink::new());
        let injector = MockInputInjector::new(policy, audit);

        let move_ev = InputEvent::MouseMove { x: 100, y: 200 };
        assert!(injector.inject(move_ev.clone()).is_ok());

        let down_ev = InputEvent::MouseDown {
            button: shared_types::MouseButton::Left,
        };
        assert!(injector.inject(down_ev.clone()).is_ok());

        let injected = injector.injected_events();
        assert_eq!(injected.len(), 2);
        assert_eq!(injected[0], move_ev);
        assert_eq!(injected[1], down_ev);
    }

    #[test]
    fn test_session_isolation() {
        // Enforce that two different injectors have completely independent events
        let policy = Arc::new(SecureInputPolicy::new());
        let audit1 = Arc::new(TestAuditSink::new());
        let audit2 = Arc::new(TestAuditSink::new());

        let injector1 = MockInputInjector::new(policy.clone(), audit1);
        let injector2 = MockInputInjector::new(policy, audit2);

        let move_ev = InputEvent::MouseMove { x: 50, y: 50 };
        injector1.inject(move_ev).unwrap();

        assert_eq!(injector1.injected_events().len(), 1);
        assert_eq!(injector2.injected_events().len(), 0); // Injector 2 remains completely isolated!
    }
}
