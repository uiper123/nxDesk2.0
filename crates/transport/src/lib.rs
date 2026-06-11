use anyhow::{Context, Result};
use protocol::Frame;
use shared_types::ConnectionConfig;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;

/// Real TCP transport for the TTGT protocol.
/// Connects to a remote host, performs handshake, and relays frames.
pub struct TcpTransport {
    config: ConnectionConfig,
    reader: Option<BufReader<tokio::io::ReadHalf<TcpStream>>>,
    writer: Option<BufWriter<tokio::io::WriteHalf<TcpStream>>>,
    connected: bool,
    frames_sent: AtomicU64,
    frames_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
}

impl TcpTransport {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            reader: None,
            writer: None,
            connected: false,
            frames_sent: AtomicU64::new(0),
            frames_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        tracing::info!("Connecting to remote host: {}", addr);

        let stream = TcpStream::connect(&addr)
            .await
            .with_context(|| format!("Failed to connect to {}", addr))?;

        // Enable TCP_NODELAY for low-latency
        stream.set_nodelay(true)?;

        let (read_half, write_half) = tokio::io::split(stream);
        self.reader = Some(BufReader::new(read_half));
        self.writer = Some(BufWriter::new(write_half));
        self.connected = true;

        tracing::info!("Successfully connected to {}", addr);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub async fn send_frame(&mut self, frame: Frame) -> Result<()> {
        let writer = self.writer.as_mut().context("Transport not connected")?;

        let bytes = frame.to_bytes();
        writer
            .write_all(&bytes)
            .await
            .context("Failed to write frame to transport")?;
        writer
            .flush()
            .await
            .context("Failed to flush transport writer")?;

        self.frames_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent
            .fetch_add(bytes.len() as u64, Ordering::Relaxed);

        Ok(())
    }

    pub async fn receive_frame(&mut self) -> Result<Frame> {
        let reader = self.reader.as_mut().context("Transport not connected")?;

        // Read 11-byte header first (TTGT magic + version + channel + length + flags)
        let mut header = [0u8; 11];
        reader
            .read_exact(&mut header)
            .await
            .context("Failed to read frame header")?;

        if &header[0..4] != b"TTGT" {
            anyhow::bail!("Invalid magic bytes in received frame");
        }

        let payload_length =
            u32::from_be_bytes([header[6], header[7], header[8], header[9]]) as usize;

        // Read payload
        let mut full_data = Vec::with_capacity(11 + payload_length);
        full_data.extend_from_slice(&header);

        if payload_length > 0 {
            let mut payload = vec![0u8; payload_length];
            reader
                .read_exact(&mut payload)
                .await
                .context("Failed to read frame payload")?;
            full_data.extend_from_slice(&payload);
        }

        let total_bytes = full_data.len() as u64;
        let frame = Frame::from_bytes(&full_data).context("Failed to parse received frame")?;

        self.frames_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received
            .fetch_add(total_bytes, Ordering::Relaxed);

        Ok(frame)
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut writer) = self.writer.take() {
            let _ = writer.flush().await;
            let _ = writer.shutdown().await;
        }
        self.reader = None;
        self.connected = false;
        tracing::info!(
            "Disconnected from {}:{}",
            self.config.host,
            self.config.port
        );
        Ok(())
    }

    /// Get transport statistics
    pub fn stats(&self) -> TransportStats {
        TransportStats {
            frames_sent: self.frames_sent.load(Ordering::Relaxed),
            frames_received: self.frames_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransportStats {
    pub frames_sent: u64,
    pub frames_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Legacy SshTransport preserved for backward compatibility
pub struct SshTransport {
    inner: TcpTransport,
}

impl SshTransport {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            inner: TcpTransport::new(config),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        self.inner.connect().await
    }

    pub async fn send_frame(&mut self, frame: Frame) -> Result<()> {
        self.inner.send_frame(frame).await
    }

    pub async fn receive_frame(&mut self) -> Result<Frame> {
        self.inner.receive_frame().await
    }
}
