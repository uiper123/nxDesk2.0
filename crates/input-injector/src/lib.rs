pub mod backends;
pub mod events;
pub mod policy;
pub mod traits;

pub use backends::mock::MockInputInjector;
pub use events::InputEvent;
pub use policy::SecureInputPolicy;
pub use traits::{InputAuditSink, InputInjector, InputPolicy, KeyboardMapper, MouseMapper};

#[cfg(target_os = "linux")]
pub use backends::x11::X11InputInjector;

#[cfg(target_os = "windows")]
pub use backends::windows::WindowsInputInjector;

use std::sync::Mutex;

/// Cross-platform low-level input injector used by the server agent's connection
/// handler. On Linux it drives X11 via XTEST; on Windows it drives the active
/// desktop via the SendInput API. The name is kept (`LegacyX11InputInjector`)
/// for backwards source compatibility with existing call sites.
///
/// The `display` field is only meaningful on Linux (e.g. ":10"). On other
/// platforms it is ignored.
pub struct LegacyX11InputInjector {
    #[allow(dead_code)]
    display: String,
}

impl LegacyX11InputInjector {
    pub fn new(display: &str) -> Self {
        Self {
            display: display.to_string(),
        }
    }
}

// ----------------------------- Linux (X11) -----------------------------
#[cfg(target_os = "linux")]
mod linux_impl {
    use super::LegacyX11InputInjector;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::ConnectionExt as _;
    use x11rb::protocol::xtest::ConnectionExt as _;

    impl LegacyX11InputInjector {
        pub fn inject_mouse_move(&self, x: u16, y: u16) -> anyhow::Result<()> {
            let (conn, _) = x11rb::connect(Some(&self.display))?;
            conn.xtest_fake_input(6, 0, 0, x11rb::NONE, x as i16, y as i16, 0)?;
            conn.flush()?;
            Ok(())
        }

        pub fn inject_mouse_click(
            &self,
            button: shared_types::MouseButton,
            pressed: bool,
        ) -> anyhow::Result<()> {
            let (conn, _) = x11rb::connect(Some(&self.display))?;
            let detail = match button {
                shared_types::MouseButton::Left => 1,
                shared_types::MouseButton::Middle => 2,
                shared_types::MouseButton::Right => 3,
                shared_types::MouseButton::None => return Ok(()),
            };
            let type_id = if pressed { 4 } else { 5 };
            conn.xtest_fake_input(type_id, detail, 0, x11rb::NONE, 0, 0, 0)?;
            conn.flush()?;
            Ok(())
        }

        pub fn inject_mouse_scroll(&self, delta: i16) -> anyhow::Result<()> {
            let (conn, _) = x11rb::connect(Some(&self.display))?;
            let detail = if delta > 0 { 4 } else { 5 };
            conn.xtest_fake_input(4, detail, 0, x11rb::NONE, 0, 0, 0)?;
            conn.xtest_fake_input(5, detail, 0, x11rb::NONE, 0, 0, 0)?;
            conn.flush()?;
            Ok(())
        }

        pub fn inject_keypress(&self, keysym: u32, pressed: bool) -> anyhow::Result<()> {
            let (conn, _) = x11rb::connect(Some(&self.display))?;
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
                let type_id = if pressed { 2 } else { 3 };
                conn.xtest_fake_input(type_id, keycode, 0, x11rb::NONE, 0, 0, 0)?;
                conn.flush()?;
            }
            Ok(())
        }
    }
}

// ----------------------------- Windows -----------------------------
#[cfg(target_os = "windows")]
mod windows_impl {
    use super::LegacyX11InputInjector;
    use crate::events::InputEvent;
    use crate::policy::SecureInputPolicy;
    use crate::traits::InputInjector;
    use crate::WindowsInputInjector;
    use std::sync::Arc;

    impl LegacyX11InputInjector {
        fn injector() -> WindowsInputInjector {
            let policy = Arc::new(SecureInputPolicy::new());
            let audit = Arc::new(crate::NoopAuditSink);
            WindowsInputInjector::new(policy, audit)
        }

        pub fn inject_mouse_move(&self, x: u16, y: u16) -> anyhow::Result<()> {
            Self::injector().inject(InputEvent::MouseMove { x, y })
        }

        pub fn inject_mouse_click(
            &self,
            button: shared_types::MouseButton,
            pressed: bool,
        ) -> anyhow::Result<()> {
            let event = if pressed {
                InputEvent::MouseDown { button }
            } else {
                InputEvent::MouseUp { button }
            };
            Self::injector().inject(event)
        }

        pub fn inject_mouse_scroll(&self, delta: i16) -> anyhow::Result<()> {
            Self::injector().inject(InputEvent::MouseScroll { delta })
        }

        pub fn inject_keypress(&self, keysym: u32, pressed: bool) -> anyhow::Result<()> {
            let event = if pressed {
                InputEvent::KeyDown { keycode: 0, keysym }
            } else {
                InputEvent::KeyUp { keycode: 0, keysym }
            };
            Self::injector().inject(event)
        }
    }
}

// ------------------- Fallback (other OSes) -------------------
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod fallback_impl {
    use super::LegacyX11InputInjector;

    impl LegacyX11InputInjector {
        pub fn inject_mouse_move(&self, _x: u16, _y: u16) -> anyhow::Result<()> {
            Ok(())
        }
        pub fn inject_mouse_click(
            &self,
            _button: shared_types::MouseButton,
            _pressed: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        pub fn inject_mouse_scroll(&self, _delta: i16) -> anyhow::Result<()> {
            Ok(())
        }
        pub fn inject_keypress(&self, _keysym: u32, _pressed: bool) -> anyhow::Result<()> {
            Ok(())
        }
    }
}

/// Audit sink that discards events. Used by the lightweight legacy injector path.
pub struct NoopAuditSink;

impl InputAuditSink for NoopAuditSink {
    fn audit_event(&self, _event_type: &str, _details: &str) {}
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
