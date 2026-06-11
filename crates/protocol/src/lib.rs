use anyhow::{bail, Result};
use thiserror::Error;

pub mod messages;

pub const MAGIC_BYTES: &[u8; 4] = b"TTGT";
pub const PROTOCOL_VERSION: u8 = 0x01;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid magic bytes")]
    InvalidMagic,
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(u8),
    #[error("Incomplete frame header")]
    IncompleteHeader,
}

#[derive(Debug, Clone)]
pub struct FrameHeader {
    pub version: u8,
    pub channel_id: u8,
    pub length: u32,
    pub flags: u8,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub header: FrameHeader,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(11 + self.payload.len());
        buffer.extend_from_slice(MAGIC_BYTES);
        buffer.push(self.header.version);
        buffer.push(self.header.channel_id);
        buffer.extend_from_slice(&self.header.length.to_be_bytes());
        buffer.push(self.header.flags);
        buffer.extend_from_slice(&self.payload);
        buffer
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 11 {
            bail!(ProtocolError::IncompleteHeader);
        }
        if &data[0..4] != MAGIC_BYTES {
            bail!(ProtocolError::InvalidMagic);
        }
        let version = data[4];
        if version != PROTOCOL_VERSION {
            bail!(ProtocolError::UnsupportedVersion(version));
        }
        let channel_id = data[5];

        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&data[6..10]);
        let length = u32::from_be_bytes(len_bytes);

        let flags = data[10];

        if data.len() < 11 + length as usize {
            bail!("Payload incomplete");
        }
        let payload = data[11..11 + length as usize].to_vec();

        Ok(Frame {
            header: FrameHeader {
                version,
                channel_id,
                length,
                flags,
            },
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use messages::control::{ClientHello, ControlMessage};
    use messages::file::FileChunkHeader;
    use messages::input::{InputEvent, KeyboardEvent, MouseEvent};
    use shared_types::MouseButton;

    #[test]
    fn test_frame_serialization() {
        let frame = Frame {
            header: FrameHeader {
                version: PROTOCOL_VERSION,
                channel_id: 2,
                length: 5,
                flags: 0,
            },
            payload: vec![1, 2, 3, 4, 5],
        };
        let bytes = frame.to_bytes();
        let parsed = Frame::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header.channel_id, 2);
        assert_eq!(parsed.payload, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_invalid_magic() {
        let bytes = vec![0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 0];
        assert!(Frame::from_bytes(&bytes).is_err());
    }

    #[test]
    fn test_control_json_serialization() {
        let msg = ControlMessage::ClientHello(ClientHello {
            client_version: "1.0.0".to_string(),
            supported_capabilities: vec!["video".to_string()],
        });
        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: ControlMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_mouse_input_binary_serialization() {
        let mouse_event = InputEvent::Mouse(MouseEvent {
            event_type: 0x01,
            button: MouseButton::Left,
            x: 800,
            y: 600,
            scroll_delta: 0,
        });
        let bytes = mouse_event.to_bytes();
        let parsed = InputEvent::from_bytes(&bytes).unwrap();
        assert_eq!(mouse_event, parsed);
    }

    #[test]
    fn test_keyboard_input_binary_serialization() {
        let key_event = InputEvent::Keyboard(KeyboardEvent {
            event_type: 0x05,
            keysym: 0xff51, // Left arrow
        });
        let bytes = key_event.to_bytes();
        let parsed = InputEvent::from_bytes(&bytes).unwrap();
        assert_eq!(key_event, parsed);
    }

    #[test]
    fn test_file_chunk_header_binary_serialization() {
        let header = FileChunkHeader {
            transfer_id: 123456789,
            offset: 4096,
        };
        let bytes = header.to_bytes();
        let parsed = FileChunkHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header, parsed);
    }
}
