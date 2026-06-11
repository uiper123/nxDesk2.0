use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    pub transfer_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub is_upload: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChunkHeader {
    pub transfer_id: u64,
    pub offset: u64,
}

impl FileChunkHeader {
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut buffer = [0u8; 16];
        buffer[0..8].copy_from_slice(&self.transfer_id.to_be_bytes());
        buffer[8..16].copy_from_slice(&self.offset.to_be_bytes());
        buffer
    }

    pub fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        if data.len() < 16 {
            anyhow::bail!("Buffer too short for FileChunkHeader");
        }
        let mut id_bytes = [0u8; 8];
        id_bytes.copy_from_slice(&data[0..8]);
        let transfer_id = u64::from_be_bytes(id_bytes);

        let mut offset_bytes = [0u8; 8];
        offset_bytes.copy_from_slice(&data[8..16]);
        let offset = u64::from_be_bytes(offset_bytes);

        Ok(Self {
            transfer_id,
            offset,
        })
    }
}
