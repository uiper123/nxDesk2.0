use crate::events::InputEvent;
use crate::traits::{InputAuditSink, InputInjector, InputPolicy};
use anyhow::{bail, Result};
use std::sync::Arc;
use tracing::warn;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_WHEEL, MOUSEINPUT, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END,
    VK_ESCAPE, VK_HOME, VK_INSERT, VK_LEFT, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT,
    VK_SHIFT, VK_SPACE, VK_TAB, VK_UP, VK_LWIN, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7,
    VK_F8, VK_F9, VK_F10, VK_F11, VK_F12,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

pub struct WindowsInputInjector {
    policy: Arc<dyn InputPolicy>,
    audit: Arc<dyn InputAuditSink>,
}

impl WindowsInputInjector {
    pub fn new(policy: Arc<dyn InputPolicy>, audit: Arc<dyn InputAuditSink>) -> Self {
        Self { policy, audit }
    }

    #[cfg(target_os = "windows")]
    fn send_mouse_move(x: u16, y: u16) -> Result<()> {
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1) as f32;
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1) as f32;
        let abs_x = ((x as f32) * 65535.0 / screen_w).round() as i32;
        let abs_y = ((y as f32) * 65535.0 / screen_h).round() as i32;
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: abs_x,
                    dy: abs_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if sent == 0 {
            bail!("SendInput mouse move failed")
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn send_mouse_button(button: shared_types::MouseButton, pressed: bool) -> Result<()> {
        let flags = match (button, pressed) {
            (shared_types::MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
            (shared_types::MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
            (shared_types::MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
            (shared_types::MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
            (shared_types::MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
            (shared_types::MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
            (shared_types::MouseButton::None, _) => return Ok(()),
        };
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if sent == 0 {
            bail!("SendInput mouse button failed")
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn send_mouse_scroll(delta: i16) -> Result<()> {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: delta as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if sent == 0 {
            bail!("SendInput mouse wheel failed")
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn x11_keysym_to_vk(keysym: u32) -> Option<VIRTUAL_KEY> {
        // Map common X11 keysyms (0xFF.. range and modifiers) to Windows virtual keys.
        let vk = match keysym {
            0xFF08 => VK_BACK,
            0xFF09 => VK_TAB,
            0xFF0D => VK_RETURN,
            0xFF1B => VK_ESCAPE,
            0xFF50 => VK_HOME,
            0xFF51 => VK_LEFT,
            0xFF52 => VK_UP,
            0xFF53 => VK_RIGHT,
            0xFF54 => VK_DOWN,
            0xFF55 => VK_PRIOR, // Page Up
            0xFF56 => VK_NEXT,  // Page Down
            0xFF57 => VK_END,
            0xFF63 => VK_INSERT,
            0xFFFF => VK_DELETE,
            0xFF20 => VK_TAB, // ISO_Lock fallback
            0x0020 => VK_SPACE,
            0xFFE1 | 0xFFE2 => VK_SHIFT,
            0xFFE3 | 0xFFE4 => VK_CONTROL,
            0xFFE9 | 0xFFEA => VK_MENU, // Alt
            0xFFEB | 0xFFEC => VK_LWIN, // Super/Win
            0xFFBE => VK_F1,
            0xFFBF => VK_F2,
            0xFFC0 => VK_F3,
            0xFFC1 => VK_F4,
            0xFFC2 => VK_F5,
            0xFFC3 => VK_F6,
            0xFFC4 => VK_F7,
            0xFFC5 => VK_F8,
            0xFFC6 => VK_F9,
            0xFFC7 => VK_F10,
            0xFFC8 => VK_F11,
            0xFFC9 => VK_F12,
            _ => return None,
        };
        Some(vk)
    }

    #[cfg(target_os = "windows")]
    fn send_vk(vk: VIRTUAL_KEY, pressed: bool) -> Result<()> {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if pressed {
                        Default::default()
                    } else {
                        KEYEVENTF_KEYUP
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if sent == 0 {
            bail!("SendInput VK failed")
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn send_keypress(keysym: u32, pressed: bool) -> Result<()> {
        // Special keys (arrows, enter, modifiers, function keys) go through VK codes.
        if let Some(vk) = Self::x11_keysym_to_vk(keysym) {
            return Self::send_vk(vk, pressed);
        }

        // Printable characters: X11 Latin keysyms match Unicode codepoints for the
        // ASCII / Latin-1 range, so we can inject them directly as Unicode.
        let unicode = char::from_u32(keysym).unwrap_or('\0');
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unicode as u16,
                    dwFlags: if pressed {
                        KEYEVENTF_UNICODE
                    } else {
                        KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if sent == 0 {
            bail!("SendInput keyboard failed")
        }
        Ok(())
    }

    fn inject_windows(&self, event: &InputEvent) -> Result<()> {
        match event {
            InputEvent::MouseMove { x, y } => Self::send_mouse_move(*x, *y),
            InputEvent::MouseDown { button } => Self::send_mouse_button(button.clone(), true),
            InputEvent::MouseUp { button } => Self::send_mouse_button(button.clone(), false),
            InputEvent::MouseScroll { delta } => Self::send_mouse_scroll(*delta),
            InputEvent::KeyDown { keysym, .. } => Self::send_keypress(*keysym, true),
            InputEvent::KeyUp { keysym, .. } => Self::send_keypress(*keysym, false),
            InputEvent::TextInput { text } => {
                for ch in text.chars() {
                    Self::send_keypress(ch as u32, true)?;
                    Self::send_keypress(ch as u32, false)?;
                }
                Ok(())
            }
            InputEvent::Hotkey { combo } => {
                warn!("Hotkey injection on Windows uses text fallback for combo: {}", combo);
                Ok(())
            }
        }
    }
}

impl InputInjector for WindowsInputInjector {
    fn inject(&self, event: InputEvent) -> Result<()> {
        if !self.policy.is_allowed(&event) {
            self.audit.audit_event(
                "INPUT_REJECTED",
                &format!("Policy blocked event: {:?}", event),
            );
            bail!("Input event rejected by security policy");
        }

        self.inject_windows(&event)
    }
}
