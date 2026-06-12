//! Tile-based delta encoder.
//!
//! The legacy pipeline re-encoded and re-sent a full-screen PNG on every frame,
//! even when only a cursor or a clock pixel changed. That wastes enormous
//! bandwidth on typical desktop work (mostly-static screens). This encoder
//! instead divides the frame into fixed tiles, hashes each tile, and only emits
//! the tiles that changed since the previous frame. A full keyframe is emitted
//! periodically (and on the first frame, on resize, or when too much of the
//! screen changed at once) so a freshly-attached or recovering client can
//! always rebuild a complete image.
//!
//! The wire format is self-describing and codec-agnostic on the receiving side:
//!
//! ```text
//! magic:   u8  = 0xD1                 // "delta v1"
//! flags:   u8                         // bit0 = keyframe
//! width:   u16 (BE)                   // full frame width
//! height:  u16 (BE)                   // full frame height
//! tile:    u16 (BE)                   // tile edge length in pixels
//! count:   u16 (BE)                   // number of tile records that follow
//! records: count × TileRecord
//!
//! TileRecord:
//!   tx:    u16 (BE)                   // tile column
//!   ty:    u16 (BE)                   // tile row
//!   len:   u32 (BE)                   // PNG byte length
//!   png:   [u8; len]                  // PNG of this tile (RGBA)
//! ```
//!
//! A keyframe simply contains every tile. The format is intentionally simple so
//! a JS/Canvas client can decode it without a heavyweight codec.

use crate::traits::{CaptureFrame, EncodedFrame, VideoEncoder};
use anyhow::{bail, Result};

pub const DELTA_MAGIC: u8 = 0xD1;
pub const FLAG_KEYFRAME: u8 = 0x01;

/// Default tile edge. 64px balances per-tile PNG overhead against the
/// granularity of change detection.
pub const DEFAULT_TILE: u32 = 64;

/// If more than this fraction of tiles changed, just send a keyframe — it is
/// cheaper than a huge pile of per-tile records and headers.
const KEYFRAME_CHANGE_RATIO: f64 = 0.6;

/// Emit a keyframe at least this often (in frames) so late joiners and lossy
/// links self-heal.
const KEYFRAME_INTERVAL: u64 = 120;

pub struct DeltaEncoder {
    tile: u32,
    prev_hashes: Vec<u64>,
    prev_w: u32,
    prev_h: u32,
    frame_count: u64,
    name: String,
}

impl DeltaEncoder {
    pub fn new(tile: u32) -> Self {
        Self {
            tile: tile.max(8),
            prev_hashes: Vec::new(),
            prev_w: 0,
            prev_h: 0,
            frame_count: 0,
            name: "Delta-Tile-PNG".to_string(),
        }
    }

    /// Encode a captured RGBA frame into a delta payload.
    pub fn encode(&mut self, frame: &CaptureFrame) -> Result<EncodedFrame> {
        let (w, h) = (frame.width, frame.height);
        if w == 0 || h == 0 {
            bail!("DeltaEncoder: zero-sized frame ({w}x{h})");
        }
        let expected = (w as usize) * (h as usize) * 4;
        if frame.rgba.len() != expected {
            bail!(
                "DeltaEncoder: buffer len {} != {expected} for {w}x{h}",
                frame.rgba.len()
            );
        }

        let cols = w.div_ceil(self.tile);
        let rows = h.div_ceil(self.tile);
        let tile_count = (cols * rows) as usize;

        let resized = w != self.prev_w || h != self.prev_h;
        let periodic = self.frame_count.is_multiple_of(KEYFRAME_INTERVAL);
        let first = self.frame_count == 0;

        let mut new_hashes = vec![0u64; tile_count];
        let mut changed: Vec<(u32, u32)> = Vec::new();

        for ty in 0..rows {
            for tx in 0..cols {
                let idx = (ty * cols + tx) as usize;
                let hash = hash_tile(&frame.rgba, w, h, tx, ty, self.tile);
                new_hashes[idx] = hash;
                let was_changed = resized
                    || first
                    || periodic
                    || self.prev_hashes.get(idx).copied() != Some(hash);
                if was_changed {
                    changed.push((tx, ty));
                }
            }
        }

        let change_ratio = changed.len() as f64 / tile_count.max(1) as f64;
        let keyframe = first || resized || periodic || change_ratio >= KEYFRAME_CHANGE_RATIO;

        // On a keyframe, (re)send every tile regardless of the diff.
        let tiles_to_send: Vec<(u32, u32)> = if keyframe {
            (0..rows)
                .flat_map(|ty| (0..cols).map(move |tx| (tx, ty)))
                .collect()
        } else {
            changed
        };

        let mut payload = Vec::with_capacity(10 + tiles_to_send.len() * 256);
        payload.push(DELTA_MAGIC);
        payload.push(if keyframe { FLAG_KEYFRAME } else { 0 });
        payload.extend_from_slice(&(w as u16).to_be_bytes());
        payload.extend_from_slice(&(h as u16).to_be_bytes());
        payload.extend_from_slice(&(self.tile as u16).to_be_bytes());
        payload.extend_from_slice(&(tiles_to_send.len() as u16).to_be_bytes());

        for (tx, ty) in &tiles_to_send {
            let (tw, th) = tile_dims(w, h, *tx, *ty, self.tile);
            let png = encode_tile_png(&frame.rgba, w, *tx, *ty, tw, th, self.tile)?;
            payload.extend_from_slice(&(*tx as u16).to_be_bytes());
            payload.extend_from_slice(&(*ty as u16).to_be_bytes());
            payload.extend_from_slice(&(png.len() as u32).to_be_bytes());
            payload.extend_from_slice(&png);
        }

        self.prev_hashes = new_hashes;
        self.prev_w = w;
        self.prev_h = h;
        self.frame_count += 1;

        Ok(EncodedFrame {
            data: payload,
            is_keyframe: keyframe,
            timestamp_ms: now_ms(),
        })
    }
}

impl VideoEncoder for DeltaEncoder {
    fn encode_frame(&mut self, raw_frame: &[u8]) -> Result<EncodedFrame> {
        // Best-effort dimension inference for callers that only have a flat
        // buffer; the typed `encode` path is strongly preferred.
        let pixels = raw_frame.len() / 4;
        let (w, h) = match pixels {
            2_073_600 => (1920u32, 1080u32),
            921_600 => (1280, 720),
            76_800 => (320, 240),
            _ => bail!("DeltaEncoder::encode_frame needs typed CaptureFrame for {pixels} px"),
        };
        self.encode(&CaptureFrame {
            rgba: raw_frame.to_vec(),
            width: w,
            height: h,
        })
    }

    fn adjust_bitrate(&mut self, _bitrate_kbps: u32) -> Result<()> {
        // Delta encoding is lossless PNG per tile; bitrate is governed by the
        // capture FPS and on-screen activity rather than a quantiser.
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

fn tile_dims(w: u32, h: u32, tx: u32, ty: u32, tile: u32) -> (u32, u32) {
    let tw = tile.min(w - tx * tile);
    let th = tile.min(h - ty * tile);
    (tw, th)
}

fn hash_tile(rgba: &[u8], w: u32, h: u32, tx: u32, ty: u32, tile: u32) -> u64 {
    // FNV-1a over the tile's pixels.
    let (tw, th) = tile_dims(w, h, tx, ty, tile);
    let mut hash: u64 = 0xcbf29ce484222325;
    for row in 0..th {
        let y = ty * tile + row;
        let start = ((y * w + tx * tile) * 4) as usize;
        let end = start + (tw * 4) as usize;
        for &b in &rgba[start..end] {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn encode_tile_png(
    rgba: &[u8],
    w: u32,
    tx: u32,
    ty: u32,
    tw: u32,
    th: u32,
    tile: u32,
) -> Result<Vec<u8>> {
    // Extract the tile into a tightly packed RGBA buffer.
    let mut buf = Vec::with_capacity((tw * th * 4) as usize);
    for row in 0..th {
        let y = ty * tile + row;
        let start = ((y * w + tx * tile) * 4) as usize;
        let end = start + (tw * 4) as usize;
        buf.extend_from_slice(&rgba[start..end]);
    }

    let mut png = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png, tw, th);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut writer = enc.write_header()?;
        writer.write_image_data(&buf)?;
    }
    Ok(png)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, color: [u8; 4]) -> CaptureFrame {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            rgba.extend_from_slice(&color);
        }
        CaptureFrame {
            rgba,
            width: w,
            height: h,
        }
    }

    #[test]
    fn first_frame_is_keyframe_with_all_tiles() {
        let mut enc = DeltaEncoder::new(64);
        let f = solid(128, 128, [10, 20, 30, 255]);
        let out = enc.encode(&f).unwrap();
        assert!(out.is_keyframe);
        // 128/64 = 2 cols × 2 rows = 4 tiles
        let count = u16::from_be_bytes([out.data[8], out.data[9]]);
        assert_eq!(count, 4);
    }

    #[test]
    fn unchanged_frame_sends_no_tiles() {
        let mut enc = DeltaEncoder::new(64);
        let f = solid(128, 128, [10, 20, 30, 255]);
        let _ = enc.encode(&f).unwrap(); // keyframe
        let out = enc.encode(&f).unwrap(); // identical
        assert!(!out.is_keyframe);
        let count = u16::from_be_bytes([out.data[8], out.data[9]]);
        assert_eq!(count, 0, "identical frame must produce zero tile records");
    }

    #[test]
    fn single_tile_change_sends_one_tile() {
        let mut enc = DeltaEncoder::new(64);
        let f1 = solid(128, 128, [0, 0, 0, 255]);
        let _ = enc.encode(&f1).unwrap();
        // Flip one pixel in the top-left tile.
        let mut f2 = f1.clone();
        f2.rgba[0] = 255;
        let out = enc.encode(&f2).unwrap();
        assert!(!out.is_keyframe);
        let count = u16::from_be_bytes([out.data[8], out.data[9]]);
        assert_eq!(count, 1);
        // The record should reference tile (0,0).
        assert_eq!(u16::from_be_bytes([out.data[10], out.data[11]]), 0);
        assert_eq!(u16::from_be_bytes([out.data[12], out.data[13]]), 0);
    }

    #[test]
    fn resize_forces_keyframe() {
        let mut enc = DeltaEncoder::new(64);
        let _ = enc.encode(&solid(128, 128, [1, 2, 3, 255])).unwrap();
        let out = enc.encode(&solid(256, 128, [1, 2, 3, 255])).unwrap();
        assert!(out.is_keyframe);
    }
}
