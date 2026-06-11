use crate::events::InputEvent;
use crate::traits::InputPolicy;
use tracing::warn;

#[derive(Default)]
pub struct SecureInputPolicy;

impl SecureInputPolicy {
    pub fn new() -> Self {
        Self
    }
}

impl InputPolicy for SecureInputPolicy {
    fn is_allowed(&self, event: &InputEvent) -> bool {
        match event {
            InputEvent::KeyDown { keysym, .. } | InputEvent::KeyUp { keysym, .. } => {
                // Block VT switching key combinations (Ctrl + Alt + F1..F12)
                // In X11, F1..F12 are usually keysyms 0xFFBE to 0xFFC9
                // Let's also check if they are combined or if we block dangerous key combos.
                // For safety, we block direct VT/SysRq keysyms.
                let sysrq_keysym = 0xFF15; // Break/SysRq
                if *keysym == sysrq_keysym {
                    warn!("Blocked SysRq input attempt.");
                    return false;
                }
                true
            }
            InputEvent::Hotkey { combo } => {
                let lower = combo.to_lowercase();
                // Block Ctrl+Alt+F1 to F12 and Ctrl+Alt+Delete
                if lower.contains("ctrl")
                    && lower.contains("alt")
                    && (lower.contains("f1")
                        || lower.contains("f2")
                        || lower.contains("f3")
                        || lower.contains("f4")
                        || lower.contains("f5")
                        || lower.contains("f6")
                        || lower.contains("f7")
                        || lower.contains("f8")
                        || lower.contains("f9")
                        || lower.contains("f10")
                        || lower.contains("f11")
                        || lower.contains("f12")
                        || lower.contains("del")
                        || lower.contains("backspace"))
                {
                    warn!("Blocked forbidden hotkey combo: {}", combo);
                    return false;
                }
                true
            }
            _ => true,
        }
    }
}
