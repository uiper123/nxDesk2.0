pub mod bitrate;
pub mod capture;
pub mod clock;
pub mod encoder;
pub mod stream;
pub mod traits;

pub use bitrate::AdaptiveBitrateController;
pub use capture::{MockCaptureSource, X11CaptureSource};
pub use clock::SimpleFrameClock;
pub use encoder::{GStreamerEncoder, MockVideoEncoder, SoftwareFallbackEncoder};
pub use stream::LocalVideoStream;
pub use traits::{
    BitrateController, CaptureSource, EncodedFrame, FrameClock, VideoEncoder, VideoMetrics,
    VideoStream,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_mock_stream() {
        let capture = Box::new(MockCaptureSource::new(320, 240));
        let encoder = Box::new(MockVideoEncoder::new("MockEnc", 2000));
        let clock = Box::new(SimpleFrameClock::new(100)); // 100 FPS for fast test

        let mut stream = LocalVideoStream::new(capture, encoder, clock);

        let frame1 = stream.next_frame().unwrap();
        assert!(frame1.is_keyframe); // first frame should be keyframe

        let frame2 = stream.next_frame().unwrap();
        assert!(!frame2.is_keyframe);

        let metrics = stream.metrics();
        assert_eq!(metrics.frames_captured, 2);
        assert_eq!(metrics.frames_dropped, 0);
        assert!(metrics.bytes_sent > 0);
    }

    #[test]
    fn test_frame_timing() {
        let mut clock = SimpleFrameClock::new(30); // 30 FPS => ~33.3ms

        let start = Instant::now();
        let _ = clock.tick(); // first tick sets last_tick
        let delta1 = clock.tick();
        let delta2 = clock.tick();

        let elapsed = start.elapsed();
        // 2 ticks at 30 FPS should take at least 60ms
        assert!(elapsed.as_millis() >= 60);
        assert!(delta1.as_millis() >= 30);
        assert!(delta2.as_millis() >= 30);
    }

    #[test]
    fn test_bitrate_controller() {
        let mut controller = AdaptiveBitrateController::new(500, 8000, 2000, 30);

        // Low latency and no loss => increase bitrate
        let b1 = controller.calculate_bitrate(25, 0.0);
        assert!(b1 > 2000);

        // High latency => decrease bitrate
        let current = controller.current_bitrate();
        let b2 = controller.calculate_bitrate(100, 0.0);
        assert!(b2 < current);

        // High loss => decrease bitrate
        let current = controller.current_bitrate();
        let b3 = controller.calculate_bitrate(25, 0.10);
        assert!(b3 < current);
    }

    #[test]
    fn test_encoder_fallback() {
        // Simulates dynamic fallback to software encoder when hardware fails
        let gstreamer_res = GStreamerEncoder::new(2000);
        let mut active_encoder: Box<dyn VideoEncoder> = match gstreamer_res {
            Ok(enc) => {
                // Try to perform a mock encoding check to see if hardware works
                let _test_raw = vec![0u8; 1000];
                if enc.name().contains("VAAPI") {
                    // Simulating hardware check success
                    Box::new(enc)
                } else {
                    Box::new(SoftwareFallbackEncoder::new(2000))
                }
            }
            Err(_) => Box::new(SoftwareFallbackEncoder::new(2000)),
        };

        assert!(active_encoder.name().contains("H264"));
        let frame = active_encoder.encode_frame(&vec![0u8; 500]).unwrap();
        assert!(!frame.data.is_empty());
    }

    #[test]
    fn test_perf_benchmark_skeleton() {
        let capture = MockCaptureSource::new(1920, 1080);
        let mut encoder = SoftwareFallbackEncoder::new(4000);

        // Encode a 1080p frame and measure duration
        let mut source = capture;
        let raw_frame = source.capture_frame().unwrap();

        let start = Instant::now();
        let frame = encoder.encode_frame(&raw_frame).unwrap();
        let duration = start.elapsed();

        assert!(!frame.data.is_empty());
        println!("1080p encoding duration: {:?}", duration);
    }
}
