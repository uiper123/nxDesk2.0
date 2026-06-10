use anyhow::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardContent {
    Text(String),
    Image(Vec<u8>),
    Html(String),
}

pub trait ClipboardProvider: Send + Sync {
    fn read(&self) -> Result<Option<ClipboardContent>>;
    fn write(&self, content: ClipboardContent) -> Result<()>;
}

pub trait ClipboardPolicy: Send + Sync {
    fn is_allowed(&self, content: &ClipboardContent) -> bool;
    fn max_size(&self) -> usize;
}

pub trait ClipboardSyncService: Send + Sync {
    fn sync_to_remote(&self, local_provider: &dyn ClipboardProvider, remote_provider: &dyn ClipboardProvider) -> Result<()>;
    fn sync_to_local(&self, remote_provider: &dyn ClipboardProvider, local_provider: &dyn ClipboardProvider) -> Result<()>;
}
