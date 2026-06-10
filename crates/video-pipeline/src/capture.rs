use anyhow::Result;
use crate::traits::CaptureSource;
use tracing::warn;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat};

pub struct MockCaptureSource {
    width: u32,
    height: u32,
    frame_count: u64,
}

impl MockCaptureSource {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            frame_count: 0,
        }
    }
}

impl CaptureSource for MockCaptureSource {
    fn capture_frame(&mut self) -> Result<Vec<u8>> {
        self.frame_count += 1;
        // Generate a fake RGBA image buffer (width * height * 4)
        let size = (self.width * self.height * 4) as usize;
        let mut buffer = vec![0u8; size];
        
        // Fill with dummy color pattern based on frame count
        let pattern = (self.frame_count % 256) as u8;
        for i in 0..(size / 4) {
            buffer[i * 4] = pattern;       // Red
            buffer[i * 4 + 1] = 128;       // Green
            buffer[i * 4 + 2] = 200;       // Blue
            buffer[i * 4 + 3] = 255;       // Alpha
        }
        Ok(buffer)
    }
}

pub struct X11CaptureSource {
    display: String,
    width: u32,
    height: u32,
}

impl X11CaptureSource {
    pub fn new(display: &str, width: u32, height: u32) -> Self {
        Self {
            display: display.to_string(),
            width,
            height,
        }
    }
}

impl CaptureSource for X11CaptureSource {
    fn capture_frame(&mut self) -> Result<Vec<u8>> {
        // Try to connect to X11 display
        let connection_res = x11rb::connect(Some(&self.display));
        match connection_res {
            Ok((conn, screen_num)) => {
                let setup = conn.setup();
                if let Some(screen) = setup.roots.get(screen_num) {
                    let root = screen.root;
                    // Request screen image
                    let get_image_cookie = conn.get_image(
                        ImageFormat::Z_PIXMAP,
                        root,
                        0,
                        0,
                        self.width as u16,
                        self.height as u16,
                        0xffffffff,
                    );
                    match get_image_cookie {
                        Ok(cookie) => {
                            match cookie.reply() {
                                Ok(reply) => {
                                    let data = reply.data;
                                    let pixels = (self.width * self.height) as usize;
                                    let mut rgba = vec![0u8; pixels * 4];
                                    
                                    if data.len() >= pixels * 4 {
                                        // 32-bit (BGRA/BGRx) image data
                                        for i in 0..pixels {
                                            rgba[i * 4] = data[i * 4 + 2];     // Red
                                            rgba[i * 4 + 1] = data[i * 4 + 1]; // Green
                                            rgba[i * 4 + 2] = data[i * 4];     // Blue
                                            rgba[i * 4 + 3] = 255;             // Alpha
                                        }
                                        return Ok(rgba);
                                    } else if data.len() >= pixels * 3 {
                                        // 24-bit (BGR) image data
                                        for i in 0..pixels {
                                            rgba[i * 4] = data[i * 3 + 2];     // Red
                                            rgba[i * 4 + 1] = data[i * 3 + 1]; // Green
                                            rgba[i * 4 + 2] = data[i * 3];     // Blue
                                            rgba[i * 4 + 3] = 255;             // Alpha
                                        }
                                        return Ok(rgba);
                                    } else {
                                        warn!(
                                            "X11 get_image returned data of unexpected size: {} bytes (expected at least {} bytes for {}x{} image). Falling back to mock.",
                                            data.len(),
                                            pixels * 3,
                                            self.width,
                                            self.height
                                        );
                                    }
                                }
                                Err(e) => {
                                    warn!("X11 get_image reply error on display {}: {:?}. Falling back to mock.", self.display, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("X11 get_image cookie error on display {}: {:?}. Falling back to mock.", self.display, e);
                        }
                    }
                } else {
                    warn!("X11 screen not found for index {}. Falling back to mock.", screen_num);
                }
            }
            Err(e) => {
                warn!("X11 connection failed on display {}: {:?}. Falling back to mock.", self.display, e);
            }
        }

        // Fallback to mock capture source
        let mut mock = MockCaptureSource::new(self.width, self.height);
        mock.capture_frame()
    }
}
