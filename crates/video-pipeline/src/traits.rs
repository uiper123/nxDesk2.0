use anyhow::Result;

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
