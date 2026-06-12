//! Physical monitor enumeration.
//!
//! On Linux this uses the X11 RANDR extension to enumerate connected outputs;
//! on Windows it uses `EnumDisplayMonitors`. When enumeration is unavailable
//! (headless, virtual display, or the extension is missing) we fall back to a
//! single synthetic monitor covering the configured capture size so the rest of
//! the pipeline always has at least one monitor to work with.

use crate::traits::Monitor;

/// Enumerate the monitors attached to `display` (the X11 display string such as
/// ":10" on Linux; ignored on Windows). Always returns at least one monitor.
pub fn enumerate(display: &str, fallback_w: u32, fallback_h: u32) -> Vec<Monitor> {
    #[cfg(target_os = "linux")]
    {
        match enumerate_x11(display) {
            Ok(mons) if !mons.is_empty() => return mons,
            Ok(_) => {}
            Err(e) => tracing::warn!("RANDR monitor enumeration failed: {e:?}; using fallback"),
        }
    }
    #[cfg(target_os = "windows")]
    {
        let _ = display;
        let mons = enumerate_windows();
        if !mons.is_empty() {
            return mons;
        }
    }

    let _ = display;
    vec![Monitor {
        index: 0,
        name: "Primary".to_string(),
        width: fallback_w.max(1),
        height: fallback_h.max(1),
        x: 0,
        y: 0,
        is_primary: true,
    }]
}

#[cfg(target_os = "linux")]
fn enumerate_x11(display: &str) -> anyhow::Result<Vec<Monitor>> {
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::ConnectionExt as _;
    use x11rb::protocol::xproto::ConnectionExt as _;

    let (conn, screen_num) = x11rb::connect(Some(display))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    // get_monitors gives us the RANDR monitor geometry directly.
    let reply = conn.randr_get_monitors(root, true)?.reply()?;
    let mut monitors = Vec::new();
    for (idx, m) in reply.monitors.iter().enumerate() {
        // Resolve the monitor's name atom to a human-readable string.
        let name = conn
            .get_atom_name(m.name)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| String::from_utf8_lossy(&r.name).to_string())
            .unwrap_or_else(|| format!("Monitor {idx}"));

        monitors.push(Monitor {
            index: idx as u32,
            name,
            width: m.width as u32,
            height: m.height as u32,
            x: m.x as i32,
            y: m.y as i32,
            is_primary: m.primary,
        });
    }
    Ok(monitors)
}

#[cfg(target_os = "windows")]
fn enumerate_windows() -> Vec<Monitor> {
    use std::cell::RefCell;
    use windows::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };
    use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

    thread_local! {
        static COLLECTED: RefCell<Vec<Monitor>> = const { RefCell::new(Vec::new()) };
    }

    unsafe extern "system" fn cb(hmon: HMONITOR, _hdc: HDC, _rect: *mut RECT, _lp: LPARAM) -> BOOL {
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            let r = mi.rcMonitor;
            let is_primary = (mi.dwFlags & MONITORINFOF_PRIMARY) != 0;
            COLLECTED.with(|c| {
                let mut v = c.borrow_mut();
                let index = v.len() as u32;
                v.push(Monitor {
                    index,
                    name: format!("Display {}", index + 1),
                    width: (r.right - r.left).max(0) as u32,
                    height: (r.bottom - r.top).max(0) as u32,
                    x: r.left,
                    y: r.top,
                    is_primary,
                });
            });
        }
        TRUE
    }

    COLLECTED.with(|c| c.borrow_mut().clear());
    unsafe {
        let _ = EnumDisplayMonitors(HDC::default(), None, Some(cb), LPARAM(0));
    }
    let mut mons = COLLECTED.with(|c| c.borrow().clone());
    // Sort so the primary monitor is index 0.
    mons.sort_by_key(|m| !m.is_primary);
    for (i, m) in mons.iter_mut().enumerate() {
        m.index = i as u32;
    }
    mons
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_returns_at_least_one_monitor() {
        // A bogus display that cannot be opened must still yield a fallback.
        let mons = enumerate(":nonexistent-display-99", 1280, 720);
        assert!(!mons.is_empty());
        assert!(mons.iter().any(|m| m.is_primary));
    }
}
