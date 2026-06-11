use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputEvent {
    MouseMove { x: u16, y: u16 },
    MouseDown { button: shared_types::MouseButton },
    MouseUp { button: shared_types::MouseButton },
    MouseScroll { delta: i16 },
    KeyDown { keycode: u32, keysym: u32 },
    KeyUp { keycode: u32, keysym: u32 },
    TextInput { text: String },
    Hotkey { combo: String },
}
