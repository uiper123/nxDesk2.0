use anyhow::Result;

/// A physical monitor that can be captured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Monitor {
    pub index: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}

/// A captured raw frame together with its true dimensions. Carrying the
/// dimensions explicitly avoids the fragile "infer width/height from buffer
/// length" heuristic the encoder used to rely on.
#[derive(Clone, Debug)]
pub struct CaptureFrame {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub is_keyframe: bool,
    pub timestamp_ms: u64,
}

#[derive(Clone, Debug, Default)]
pub struct VideoMetrics {
    pub frames_captured: u64,
    pub frames_dropped: u64,
    pub bytes_sent: u64,
    pub average_latency_ms: f64,
}

pub trait CaptureSource: Send + Sync {
    fn capture_frame(&mut self) -> Result<Vec<u8>>;

    /// Capture a frame along with its real dimensions. The default
    /// implementation falls back to `capture_frame` plus the source's
    /// configured dimensions, so existing sources keep working unchanged.
    fn capture(&mut self) -> Result<CaptureFrame> {
        let (w, h) = self.dimensions();
        let rgba = self.capture_frame()?;
        Ok(CaptureFrame {
            rgba,
            width: w,
            height: h,
        })
    }

    /// Configured capture dimensions (width, height). Sources that do not know
    /// their dimensions ahead of time may return (0, 0).
    fn dimensions(&self) -> (u32, u32) {
        (0, 0)
    }
}

pub trait VideoEncoder: Send + Sync {
    fn encode_frame(&mut self, raw_frame: &[u8]) -> Result<EncodedFrame>;
    fn adjust_bitrate(&mut self, bitrate_kbps: u32) -> Result<()>;
    fn name(&self) -> &str;
}

pub trait BitrateController: Send + Sync {
    fn calculate_bitrate(&mut self, current_latency_ms: u64, packet_loss_rate: f64) -> u32;
    fn current_bitrate(&self) -> u32;
}

pub trait FrameClock: Send + Sync {
    fn tick(&mut self) -> std::time::Duration;
    fn fps(&self) -> u32;
}

pub trait VideoStream: Send + Sync {
    fn next_frame(&mut self) -> Result<EncodedFrame>;
    fn metrics(&self) -> VideoMetrics;
}
