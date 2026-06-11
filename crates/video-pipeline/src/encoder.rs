use crate::traits::{EncodedFrame, VideoEncoder};
use anyhow::Result;
use gstreamer::prelude::*;
use tracing::info;
use tracing::warn;

pub struct MockVideoEncoder {
    name: String,
    bitrate_kbps: u32,
    frame_count: u64,
}

impl MockVideoEncoder {
    pub fn new(name: &str, initial_bitrate: u32) -> Self {
        Self {
            name: name.to_string(),
            bitrate_kbps: initial_bitrate,
            frame_count: 0,
        }
    }
}

fn infer_rgba_dimensions(raw_frame: &[u8]) -> Result<(u32, u32)> {
    if !raw_frame.len().is_multiple_of(4) {
        anyhow::bail!(
            "RGBA frame size must be divisible by 4, got {}",
            raw_frame.len()
        );
    }

    let pixels = raw_frame.len() / 4;
    let dimensions = match pixels {
        2073600 => (1920, 1080),
        921600 => (1280, 720),
        76800 => (320, 240),
        _ => {
            let height = ((pixels as f64) / 1.7777777).sqrt() as u32;
            if height == 0 {
                anyhow::bail!("Unable to infer frame dimensions for {} pixels", pixels);
            }
            let width = pixels as u32 / height;
            (width, height)
        }
    };

    Ok(dimensions)
}

fn rgba_to_png(raw_frame: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let expected_len = width as usize * height as usize * 4;
    if raw_frame.len() != expected_len {
        anyhow::bail!(
            "RGBA frame length mismatch: got {}, expected {} for {}x{}",
            raw_frame.len(),
            expected_len,
            width,
            height
        );
    }

    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(raw_frame)?;
    }
    Ok(png_data)
}

impl VideoEncoder for MockVideoEncoder {
    fn encode_frame(&mut self, raw_frame: &[u8]) -> Result<EncodedFrame> {
        self.frame_count += 1;
        let (width, height) = infer_rgba_dimensions(raw_frame)?;
        let png_data = rgba_to_png(raw_frame, width, height)?;
        let is_keyframe = self.frame_count % 30 == 1;

        Ok(EncodedFrame {
            data: png_data,
            is_keyframe,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }

    fn adjust_bitrate(&mut self, bitrate_kbps: u32) -> Result<()> {
        self.bitrate_kbps = bitrate_kbps;
        info!(
            "Encoder [{}] adjusted bitrate to {} kbps",
            self.name, bitrate_kbps
        );
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub struct GStreamerEncoder {
    pipeline: Option<gstreamer::Pipeline>,
    appsrc: Option<gstreamer::Element>,
    appsink: Option<gstreamer::Element>,
    initial_bitrate: u32,
    frame_count: u64,
    fallback: MockVideoEncoder,
    use_fallback: bool,
}

impl GStreamerEncoder {
    pub fn new(initial_bitrate: u32) -> Result<Self> {
        info!("Initializing GStreamer encoder backend...");
        let use_fallback = match gstreamer::init() {
            Ok(_) => false,
            Err(e) => {
                warn!(
                    "GStreamer initialization failed: {:?}. Using fallback mock encoder.",
                    e
                );
                true
            }
        };

        Ok(Self {
            pipeline: None,
            appsrc: None,
            appsink: None,
            initial_bitrate,
            frame_count: 0,
            fallback: MockVideoEncoder::new("GStreamer-H264-VAAPI-Fallback", initial_bitrate),
            use_fallback,
        })
    }

    fn init_pipeline(&mut self, width: u32, height: u32) -> Result<()> {
        let vaapi_str = format!(
            "appsrc name=src format=time caps=video/x-raw,format=RGBA,width={},height={},framerate=30/1 ! videoconvert ! vaapih264enc name=enc bitrate={} ! appsink name=sink emit-signals=true sync=false",
            width, height, self.initial_bitrate
        );

        info!("Trying to launch GStreamer VAAPI pipeline: {}", vaapi_str);
        let pipeline = match gstreamer::parse::launch(&vaapi_str) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "Failed to create GStreamer VAAPI pipeline: {:?}. Trying Software H.264...",
                    e
                );
                let x264_str = format!(
                    "appsrc name=src format=time caps=video/x-raw,format=RGBA,width={},height={},framerate=30/1 ! videoconvert ! x264enc name=enc tune=zerolatency speed-preset=ultrafast bitrate={} ! appsink name=sink emit-signals=true sync=false",
                    width, height, self.initial_bitrate
                );
                match gstreamer::parse::launch(&x264_str) {
                    Ok(p) => p,
                    Err(err) => {
                        warn!("Failed to create GStreamer x264enc pipeline: {:?}. Falling back to mock encoder.", err);
                        self.use_fallback = true;
                        return Ok(());
                    }
                }
            }
        };

        let pipeline = pipeline
            .dynamic_cast::<gstreamer::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to cast GStreamer element to Pipeline"))?;

        let appsrc = pipeline
            .by_name("src")
            .ok_or_else(|| anyhow::anyhow!("Failed to find appsrc 'src' in pipeline"))?;
        let appsink = pipeline
            .by_name("sink")
            .ok_or_else(|| anyhow::anyhow!("Failed to find appsink 'sink' in pipeline"))?;

        pipeline.set_state(gstreamer::State::Playing)?;

        self.pipeline = Some(pipeline);
        self.appsrc = Some(appsrc);
        self.appsink = Some(appsink);
        Ok(())
    }
}

impl VideoEncoder for GStreamerEncoder {
    fn encode_frame(&mut self, raw_frame: &[u8]) -> Result<EncodedFrame> {
        if self.use_fallback {
            return self.fallback.encode_frame(raw_frame);
        }

        if self.pipeline.is_none() {
            let (width, height) = infer_rgba_dimensions(raw_frame)?;

            if let Err(e) = self.init_pipeline(width, height) {
                warn!(
                    "GStreamer pipeline initialization failed: {:?}. Switching to fallback.",
                    e
                );
                self.use_fallback = true;
                return self.fallback.encode_frame(raw_frame);
            }
        }

        self.frame_count += 1;

        let mut buffer = gstreamer::Buffer::with_size(raw_frame.len()).map_err(|_| {
            anyhow::anyhow!(
                "Failed to allocate GStreamer buffer of size {}",
                raw_frame.len()
            )
        })?;
        {
            let buffer_ref = buffer
                .get_mut()
                .ok_or_else(|| anyhow::anyhow!("Failed to get mutable buffer reference"))?;
            let mut map = buffer_ref
                .map_writable()
                .map_err(|_| anyhow::anyhow!("Failed to map GStreamer buffer as writable"))?;
            map.copy_from_slice(raw_frame);
        }

        if let Some(appsrc) = &self.appsrc {
            let flow_ret = appsrc.emit_by_name::<gstreamer::FlowReturn>("push-buffer", &[&buffer]);
            if flow_ret != gstreamer::FlowReturn::Ok {
                warn!(
                    "GStreamer appsrc push-buffer returned: {:?}. Falling back.",
                    flow_ret
                );
                self.use_fallback = true;
                return self.fallback.encode_frame(raw_frame);
            }
        }

        if let Some(appsink) = &self.appsink {
            let sample = appsink.emit_by_name::<Option<gstreamer::Sample>>("pull-sample", &[]);
            if let Some(sample) = sample {
                if let Some(buffer) = sample.buffer() {
                    let map = buffer.map_readable().map_err(|_| {
                        anyhow::anyhow!("Failed to map GStreamer buffer as readable")
                    })?;
                    let data = map.to_vec();
                    let is_keyframe = !buffer.flags().contains(gstreamer::BufferFlags::DELTA_UNIT);
                    let timestamp_ms = buffer.pts().map(|pts| pts.mseconds()).unwrap_or(0);
                    return Ok(EncodedFrame {
                        data,
                        is_keyframe,
                        timestamp_ms,
                    });
                }
            } else {
                warn!("GStreamer appsink pull-sample returned None. Falling back.");
                self.use_fallback = true;
                return self.fallback.encode_frame(raw_frame);
            }
        }

        self.fallback.encode_frame(raw_frame)
    }

    fn adjust_bitrate(&mut self, bitrate_kbps: u32) -> Result<()> {
        if self.use_fallback {
            return self.fallback.adjust_bitrate(bitrate_kbps);
        }

        if let Some(pipeline) = &self.pipeline {
            if let Some(enc) = pipeline.by_name("enc") {
                enc.set_property("bitrate", bitrate_kbps);
                info!(
                    "Successfully adjusted GStreamer encoder bitrate to {} kbps",
                    bitrate_kbps
                );
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        if self.use_fallback {
            self.fallback.name()
        } else {
            "GStreamer-H264-VAAPI"
        }
    }
}

impl Drop for GStreamerEncoder {
    fn drop(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            let _ = pipeline.set_state(gstreamer::State::Null);
        }
    }
}

pub struct SoftwareFallbackEncoder {
    pipeline: Option<gstreamer::Pipeline>,
    appsrc: Option<gstreamer::Element>,
    appsink: Option<gstreamer::Element>,
    frame_count: u64,
    fallback: MockVideoEncoder,
    use_fallback: bool,
}

impl SoftwareFallbackEncoder {
    pub fn new(initial_bitrate: u32) -> Self {
        Self {
            pipeline: None,
            appsrc: None,
            appsink: None,
            frame_count: 0,
            fallback: MockVideoEncoder::new("OpenH264-Software-Fallback-Fallback", initial_bitrate),
            use_fallback: true,
        }
    }

    fn init_pipeline(&mut self, width: u32, height: u32) -> Result<()> {
        let jpeg_str = format!(
            "appsrc name=src format=time caps=video/x-raw,format=RGBA,width={},height={},framerate=30/1 ! videoconvert ! jpegenc name=enc quality=50 ! appsink name=sink emit-signals=true sync=false",
            width, height
        );

        info!(
            "Trying to launch GStreamer software jpegenc pipeline: {}",
            jpeg_str
        );
        let pipeline = match gstreamer::parse::launch(&jpeg_str) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to create GStreamer jpegenc pipeline: {:?}. Falling back to mock encoder.", e);
                self.use_fallback = true;
                return Ok(());
            }
        };

        let pipeline = pipeline
            .dynamic_cast::<gstreamer::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to cast GStreamer element to Pipeline"))?;

        let appsrc = pipeline
            .by_name("src")
            .ok_or_else(|| anyhow::anyhow!("Failed to find appsrc 'src' in pipeline"))?;
        let appsink = pipeline
            .by_name("sink")
            .ok_or_else(|| anyhow::anyhow!("Failed to find appsink 'sink' in pipeline"))?;

        pipeline.set_state(gstreamer::State::Playing)?;

        self.pipeline = Some(pipeline);
        self.appsrc = Some(appsrc);
        self.appsink = Some(appsink);
        Ok(())
    }
}

impl VideoEncoder for SoftwareFallbackEncoder {
    fn encode_frame(&mut self, raw_frame: &[u8]) -> Result<EncodedFrame> {
        if self.use_fallback {
            return self.fallback.encode_frame(raw_frame);
        }

        if self.pipeline.is_none() {
            let (width, height) = infer_rgba_dimensions(raw_frame)?;

            if let Err(e) = self.init_pipeline(width, height) {
                warn!("GStreamer software pipeline initialization failed: {:?}. Switching to fallback.", e);
                self.use_fallback = true;
                return self.fallback.encode_frame(raw_frame);
            }
        }

        self.frame_count += 1;

        let mut buffer = gstreamer::Buffer::with_size(raw_frame.len()).map_err(|_| {
            anyhow::anyhow!(
                "Failed to allocate GStreamer buffer of size {}",
                raw_frame.len()
            )
        })?;
        {
            let buffer_ref = buffer
                .get_mut()
                .ok_or_else(|| anyhow::anyhow!("Failed to get mutable buffer reference"))?;
            let mut map = buffer_ref
                .map_writable()
                .map_err(|_| anyhow::anyhow!("Failed to map GStreamer buffer as writable"))?;
            map.copy_from_slice(raw_frame);
        }

        if let Some(appsrc) = &self.appsrc {
            let flow_ret = appsrc.emit_by_name::<gstreamer::FlowReturn>("push-buffer", &[&buffer]);
            if flow_ret != gstreamer::FlowReturn::Ok {
                warn!(
                    "GStreamer appsrc push-buffer returned: {:?}. Falling back.",
                    flow_ret
                );
                self.use_fallback = true;
                return self.fallback.encode_frame(raw_frame);
            }
        }

        if let Some(appsink) = &self.appsink {
            let sample = appsink.emit_by_name::<Option<gstreamer::Sample>>("pull-sample", &[]);
            if let Some(sample) = sample {
                if let Some(buffer) = sample.buffer() {
                    let map = buffer.map_readable().map_err(|_| {
                        anyhow::anyhow!("Failed to map GStreamer buffer as readable")
                    })?;
                    let data = map.to_vec();
                    let is_keyframe = !buffer.flags().contains(gstreamer::BufferFlags::DELTA_UNIT);
                    let timestamp_ms = buffer.pts().map(|pts| pts.mseconds()).unwrap_or(0);
                    return Ok(EncodedFrame {
                        data,
                        is_keyframe,
                        timestamp_ms,
                    });
                }
            } else {
                warn!("GStreamer appsink pull-sample returned None. Falling back.");
                self.use_fallback = true;
                return self.fallback.encode_frame(raw_frame);
            }
        }

        self.fallback.encode_frame(raw_frame)
    }

    fn adjust_bitrate(&mut self, bitrate_kbps: u32) -> Result<()> {
        if self.use_fallback {
            return self.fallback.adjust_bitrate(bitrate_kbps);
        }

        if let Some(pipeline) = &self.pipeline {
            if let Some(enc) = pipeline.by_name("enc") {
                enc.set_property("bitrate", bitrate_kbps);
                info!(
                    "Successfully adjusted GStreamer software encoder bitrate to {} kbps",
                    bitrate_kbps
                );
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        if self.use_fallback {
            self.fallback.name()
        } else {
            "OpenH264-Software-Fallback"
        }
    }
}

impl Drop for SoftwareFallbackEncoder {
    fn drop(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            let _ = pipeline.set_state(gstreamer::State::Null);
        }
    }
}
