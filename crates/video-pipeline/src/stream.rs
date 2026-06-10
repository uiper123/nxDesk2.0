use anyhow::Result;
use crate::traits::{VideoStream, CaptureSource, VideoEncoder, FrameClock, EncodedFrame, VideoMetrics};
use std::time::Instant;

pub struct LocalVideoStream {
    capture: Box<dyn CaptureSource>,
    encoder: Box<dyn VideoEncoder>,
    clock: Box<dyn FrameClock>,
    metrics: VideoMetrics,
}

impl LocalVideoStream {
    pub fn new(
        capture: Box<dyn CaptureSource>,
        encoder: Box<dyn VideoEncoder>,
        clock: Box<dyn FrameClock>,
    ) -> Self {
        Self {
            capture,
            encoder,
            clock,
            metrics: VideoMetrics::default(),
        }
    }

    pub fn encoder_mut(&mut self) -> &mut dyn VideoEncoder {
        self.encoder.as_mut()
    }
}

impl VideoStream for LocalVideoStream {
    fn next_frame(&mut self) -> Result<EncodedFrame> {
        // 1. Tick the frame clock to enforce constant rate (e.g. 30 FPS)
        let _delta = self.clock.tick();

        let start = Instant::now();

        // 2. Capture raw frame
        let raw = self.capture.capture_frame();
        if raw.is_err() {
            self.metrics.frames_dropped += 1;
            return Err(raw.unwrap_err());
        }
        let raw_data = raw.unwrap();

        // 3. Encode raw frame
        let frame = self.encoder.encode_frame(&raw_data)?;

        // 4. Update metrics
        self.metrics.frames_captured += 1;
        self.metrics.bytes_sent += frame.data.len() as u64;
        
        let latency = start.elapsed().as_millis() as f64;
        self.metrics.average_latency_ms = (self.metrics.average_latency_ms 
            * (self.metrics.frames_captured - 1) as f64 
            + latency) / self.metrics.frames_captured as f64;

        Ok(frame)
    }

    fn metrics(&self) -> VideoMetrics {
        self.metrics.clone()
    }
}
