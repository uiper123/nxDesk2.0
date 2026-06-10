use anyhow::{bail, Result};
use std::sync::Arc;
use crate::traits::{InputInjector, InputPolicy, InputAuditSink};
use crate::events::InputEvent;
use tracing::{info, warn};
use x11rb::connection::Connection;
use x11rb::protocol::xtest::ConnectionExt as _;

pub struct X11InputInjector {
    display: String,
    policy: Arc<dyn InputPolicy>,
    audit: Arc<dyn InputAuditSink>,
}

impl X11InputInjector {
    pub fn new(display: &str, policy: Arc<dyn InputPolicy>, audit: Arc<dyn InputAuditSink>) -> Self {
        Self {
            display: display.to_string(),
            policy,
            audit,
        }
    }

    fn inject_x11(&self, event: &InputEvent) -> Result<()> {
        // Try connecting to the specified X11 display
        let (conn, _) = match x11rb::connect(Some(&self.display)) {
            Ok(c) => c,
            Err(e) => {
                warn!("X11 Connection failed on display {}: {:?}. Falling back to logging only.", self.display, e);
                return Ok(());
            }
        };

        match event {
            InputEvent::MouseMove { x, y } => {
                conn.xtest_fake_input(
                    6, // MotionNotify
                    0,
                    0,
                    x11rb::NONE,
                    *x as i16,
                    *y as i16,
                    0,
                )?;
                conn.flush()?;
            }
            InputEvent::MouseDown { button } => {
                let detail = match button {
                    shared_types::MouseButton::Left => 1,
                    shared_types::MouseButton::Middle => 2,
                    shared_types::MouseButton::Right => 3,
                    shared_types::MouseButton::None => return Ok(()),
                };
                conn.xtest_fake_input(
                    4, // ButtonPress
                    detail,
                    0,
                    x11rb::NONE,
                    0, 0, 0,
                )?;
                conn.flush()?;
            }
            InputEvent::MouseUp { button } => {
                let detail = match button {
                    shared_types::MouseButton::Left => 1,
                    shared_types::MouseButton::Middle => 2,
                    shared_types::MouseButton::Right => 3,
                    shared_types::MouseButton::None => return Ok(()),
                };
                conn.xtest_fake_input(
                    5, // ButtonRelease
                    detail,
                    0,
                    x11rb::NONE,
                    0, 0, 0,
                )?;
                conn.flush()?;
            }
            InputEvent::MouseScroll { delta } => {
                // Scroll up = Button 4, Scroll down = Button 5
                let detail = if *delta > 0 { 4 } else { 5 };
                conn.xtest_fake_input(
                    4, // ButtonPress
                    detail,
                    0,
                    x11rb::NONE,
                    0, 0, 0,
                )?;
                conn.xtest_fake_input(
                    5, // ButtonRelease
                    detail,
                    0,
                    x11rb::NONE,
                    0, 0, 0,
                )?;
                conn.flush()?;
            }
            InputEvent::KeyDown { keycode, .. } => {
                conn.xtest_fake_input(
                    2, // KeyPress
                    *keycode as u8,
                    0,
                    x11rb::NONE,
                    0, 0, 0,
                )?;
                conn.flush()?;
            }
            InputEvent::KeyUp { keycode, .. } => {
                conn.xtest_fake_input(
                    3, // KeyRelease
                    *keycode as u8,
                    0,
                    x11rb::NONE,
                    0, 0, 0,
                )?;
                conn.flush()?;
            }
            InputEvent::TextInput { text } => {
                info!("TextInput injection of '{}' is handled key-by-key", text);
            }
            InputEvent::Hotkey { combo } => {
                info!("Hotkey combo injection of '{}' is handled key-by-key", combo);
            }
        }

        Ok(())
    }
}

impl InputInjector for X11InputInjector {
    fn inject(&self, event: InputEvent) -> Result<()> {
        if !self.policy.is_allowed(&event) {
            self.audit.audit_event("INPUT_REJECTED", &format!("Policy blocked event: {:?}", event));
            bail!("Input event rejected by security policy");
        }

        self.inject_x11(&event)
    }
}
