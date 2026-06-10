use anyhow::{Result, bail};
use shared_types::MouseButton;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Mouse(MouseEvent),
    Keyboard(KeyboardEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseEvent {
    pub event_type: u8, // 0x01: Move, 0x02: Press, 0x03: Release, 0x04: Scroll
    pub button: MouseButton,
    pub x: u16,
    pub y: u16,
    pub scroll_delta: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardEvent {
    pub event_type: u8, // 0x05: Press, 0x06: Release
    pub keysym: u32,
}

impl InputEvent {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        match self {
            InputEvent::Mouse(m) => {
                buffer.push(m.event_type);
                let btn = match m.button {
                    MouseButton::None => 0x00,
                    MouseButton::Left => 0x01,
                    MouseButton::Right => 0x02,
                    MouseButton::Middle => 0x03,
                };
                buffer.push(btn);
                buffer.extend_from_slice(&m.x.to_be_bytes());
                buffer.extend_from_slice(&m.y.to_be_bytes());
                buffer.extend_from_slice(&m.scroll_delta.to_be_bytes());
            }
            InputEvent::Keyboard(k) => {
                buffer.push(k.event_type);
                buffer.extend_from_slice(&k.keysym.to_be_bytes());
            }
        }
        buffer
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            bail!("Input payload empty");
        }
        let event_type = data[0];
        match event_type {
            0x01..=0x04 => {
                if data.len() < 8 {
                    bail!("Mouse event too short");
                }
                let button = match data[1] {
                    0x00 => MouseButton::None,
                    0x01 => MouseButton::Left,
                    0x02 => MouseButton::Right,
                    0x03 => MouseButton::Middle,
                    _ => MouseButton::None,
                };
                
                let mut x_bytes = [0u8; 2];
                x_bytes.copy_from_slice(&data[2..4]);
                let x = u16::from_be_bytes(x_bytes);
                
                let mut y_bytes = [0u8; 2];
                y_bytes.copy_from_slice(&data[4..6]);
                let y = u16::from_be_bytes(y_bytes);
                
                let mut scroll_bytes = [0u8; 2];
                scroll_bytes.copy_from_slice(&data[6..8]);
                let scroll_delta = i16::from_be_bytes(scroll_bytes);
                
                Ok(InputEvent::Mouse(MouseEvent {
                    event_type,
                    button,
                    x,
                    y,
                    scroll_delta,
                }))
            }
            0x05..=0x06 => {
                if data.len() < 5 {
                    bail!("Keyboard event too short");
                }
                let mut keysym_bytes = [0u8; 4];
                keysym_bytes.copy_from_slice(&data[1..5]);
                let keysym = u32::from_be_bytes(keysym_bytes);
                
                Ok(InputEvent::Keyboard(KeyboardEvent {
                    event_type,
                    keysym,
                }))
            }
            _ => bail!("Unknown input event type: {}", event_type),
        }
    }
}
