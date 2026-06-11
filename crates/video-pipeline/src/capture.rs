use crate::traits::CaptureSource;
use anyhow::Result;

#[cfg(target_os = "linux")]
use tracing::warn;
#[cfg(target_os = "linux")]
use x11rb::connection::Connection;
#[cfg(target_os = "linux")]
use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat};

/// A deterministic synthetic capture source. Used for tests and as a last-resort
/// fallback when no real capture backend is available on the host.
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
            buffer[i * 4] = pattern; // Red
            buffer[i * 4 + 1] = 128; // Green
            buffer[i * 4 + 2] = 200; // Blue
            buffer[i * 4 + 3] = 255; // Alpha
        }
        Ok(buffer)
    }
}

// ----------------------------- Linux (X11) -----------------------------
#[cfg(target_os = "linux")]
pub struct X11CaptureSource {
    display: String,
    width: u32,
    height: u32,
}

#[cfg(target_os = "linux")]
impl X11CaptureSource {
    pub fn new(display: &str, width: u32, height: u32) -> Self {
        Self {
            display: display.to_string(),
            width,
            height,
        }
    }
}

#[cfg(target_os = "linux")]
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
                                            rgba[i * 4] = data[i * 4 + 2]; // Red
                                            rgba[i * 4 + 1] = data[i * 4 + 1]; // Green
                                            rgba[i * 4 + 2] = data[i * 4]; // Blue
                                            rgba[i * 4 + 3] = 255; // Alpha
                                        }
                                        return Ok(rgba);
                                    } else if data.len() >= pixels * 3 {
                                        // 24-bit (BGR) image data
                                        for i in 0..pixels {
                                            rgba[i * 4] = data[i * 3 + 2]; // Red
                                            rgba[i * 4 + 1] = data[i * 3 + 1]; // Green
                                            rgba[i * 4 + 2] = data[i * 3]; // Blue
                                            rgba[i * 4 + 3] = 255; // Alpha
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
                    warn!(
                        "X11 screen not found for index {}. Falling back to mock.",
                        screen_num
                    );
                }
            }
            Err(e) => {
                warn!(
                    "X11 connection failed on display {}: {:?}. Falling back to mock.",
                    self.display, e
                );
            }
        }

        // Fallback to mock capture source
        let mut mock = MockCaptureSource::new(self.width, self.height);
        mock.capture_frame()
    }
}

// ----------------------------- Windows (GDI) -----------------------------
#[cfg(target_os = "windows")]
pub struct WindowsCaptureSource {
    width: u32,
    height: u32,
}

#[cfg(target_os = "windows")]
impl WindowsCaptureSource {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    fn capture_gdi(&self) -> Result<Vec<u8>> {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
            GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
            DIB_RGB_COLORS, HGDIOBJ, SRCCOPY,
        };
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

        unsafe {
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let cap_w = if self.width > 0 {
                self.width as i32
            } else {
                screen_w
            }
            .min(screen_w.max(1));
            let cap_h = if self.height > 0 {
                self.height as i32
            } else {
                screen_h
            }
            .min(screen_h.max(1));

            let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
            if hdc_screen.is_invalid() {
                anyhow::bail!("GetDC(screen) failed");
            }
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_invalid() {
                ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
                anyhow::bail!("CreateCompatibleDC failed");
            }
            let hbm = CreateCompatibleBitmap(hdc_screen, cap_w, cap_h);
            if hbm.is_invalid() {
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
                anyhow::bail!("CreateCompatibleBitmap failed");
            }
            let old = SelectObject(hdc_mem, HGDIOBJ(hbm.0));

            let blt_ok = BitBlt(hdc_mem, 0, 0, cap_w, cap_h, hdc_screen, 0, 0, SRCCOPY).is_ok();

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: cap_w,
                    biHeight: -cap_h, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let pixels = (cap_w * cap_h) as usize;
            let mut bgra = vec![0u8; pixels * 4];
            let lines = GetDIBits(
                hdc_mem,
                hbm,
                0,
                cap_h as u32,
                Some(bgra.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // Cleanup
            SelectObject(hdc_mem, old);
            let _ = DeleteObject(HGDIOBJ(hbm.0));
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

            if !blt_ok || lines == 0 {
                anyhow::bail!("BitBlt/GetDIBits failed");
            }

            // Convert BGRA -> RGBA
            let mut rgba = vec![0u8; pixels * 4];
            for i in 0..pixels {
                rgba[i * 4] = bgra[i * 4 + 2]; // R
                rgba[i * 4 + 1] = bgra[i * 4 + 1]; // G
                rgba[i * 4 + 2] = bgra[i * 4]; // B
                rgba[i * 4 + 3] = 255; // A
            }
            Ok(rgba)
        }
    }
}

#[cfg(target_os = "windows")]
impl CaptureSource for WindowsCaptureSource {
    fn capture_frame(&mut self) -> Result<Vec<u8>> {
        match self.capture_gdi() {
            Ok(rgba) => Ok(rgba),
            Err(e) => {
                tracing::warn!("Windows GDI capture failed: {:?}. Falling back to mock.", e);
                let mut mock = MockCaptureSource::new(self.width, self.height);
                mock.capture_frame()
            }
        }
    }
}

/// Construct the best available screen capture source for the current platform.
///
/// `display` is only meaningful on Linux (X11 display string such as ":10").
/// On Windows the primary display is captured via GDI; on other platforms a
/// synthetic mock source is returned.
pub fn make_capture_source(display: &str, width: u32, height: u32) -> Box<dyn CaptureSource> {
    #[cfg(target_os = "linux")]
    {
        let _ = display;
        return Box::new(X11CaptureSource::new(display, width, height));
    }
    #[cfg(target_os = "windows")]
    {
        let _ = display;
        return Box::new(WindowsCaptureSource::new(width, height));
    }
    #[allow(unreachable_code)]
    {
        let _ = display;
        Box::new(MockCaptureSource::new(width, height))
    }
}
