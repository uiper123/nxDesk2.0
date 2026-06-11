/**
 * Decoder for the agent's tile-based delta video protocol (see
 * `crates/video-pipeline/src/delta.rs`). The agent only re-sends screen tiles
 * that changed since the previous frame, with periodic keyframes that contain
 * every tile. This module parses that wire format and paints the tiles onto a
 * persistent canvas, so the client reconstructs a full image while receiving a
 * fraction of the bytes a full-frame PNG stream would cost.
 *
 * Wire format (big-endian), matching the Rust encoder exactly:
 *
 *   magic:  u8  = 0xD1
 *   flags:  u8           // bit0 = keyframe
 *   width:  u16          // full frame width
 *   height: u16          // full frame height
 *   tile:   u16          // tile edge length
 *   count:  u16          // number of tile records
 *   records: count × { tx:u16, ty:u16, len:u32, png:[len] }
 */

export const DELTA_MAGIC = 0xd1;
export const FLAG_KEYFRAME = 0x01;

export interface DeltaTile {
  tx: number;
  ty: number;
  png: Uint8Array;
}

export interface DeltaFrame {
  keyframe: boolean;
  width: number;
  height: number;
  tile: number;
  tiles: DeltaTile[];
}

/** Returns true if `data` looks like a delta payload (cheap magic-byte check). */
export function isDeltaFrame(data: Uint8Array): boolean {
  return data.length >= 10 && data[0] === DELTA_MAGIC;
}

/**
 * Parse a delta payload into a structured frame. Throws on a malformed buffer
 * (truncated header or a tile record that runs past the end of the buffer) so
 * callers can fall back to requesting a fresh keyframe.
 */
export function parseDeltaFrame(data: Uint8Array): DeltaFrame {
  if (data.length < 10) {
    throw new Error(`delta frame too short: ${data.length} bytes`);
  }
  if (data[0] !== DELTA_MAGIC) {
    throw new Error(`bad delta magic: 0x${data[0].toString(16)}`);
  }

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const flags = data[1];
  const width = view.getUint16(2, false);
  const height = view.getUint16(4, false);
  const tile = view.getUint16(6, false);
  const count = view.getUint16(8, false);

  const tiles: DeltaTile[] = [];
  let off = 10;
  for (let i = 0; i < count; i++) {
    if (off + 8 > data.length) {
      throw new Error(`delta record ${i} header overruns buffer`);
    }
    const tx = view.getUint16(off, false);
    const ty = view.getUint16(off + 2, false);
    const len = view.getUint32(off + 4, false);
    off += 8;
    if (off + len > data.length) {
      throw new Error(`delta record ${i} payload overruns buffer`);
    }
    tiles.push({ tx, ty, png: data.subarray(off, off + len) });
    off += len;
  }

  return {
    keyframe: (flags & FLAG_KEYFRAME) !== 0,
    width,
    height,
    tile,
    tiles,
  };
}

/**
 * Paints a parsed delta frame onto a 2D canvas context, drawing each tile's PNG
 * at its (tx, ty) position. Resizes the canvas to match a keyframe's reported
 * dimensions. Returns a promise that resolves once all tiles are drawn.
 *
 * Decoding is done via `createImageBitmap`, which is available in browsers and
 * in the Tauri webview; tests inject a stub.
 */
export interface BitmapDecoder {
  (png: Uint8Array): Promise<{ width: number; height: number; close?: () => void }>;
}

export async function applyDeltaFrame(
  ctx: CanvasRenderingContext2D,
  frame: DeltaFrame,
  decode: BitmapDecoder,
  drawBitmap: (
    ctx: CanvasRenderingContext2D,
    bmp: { width: number; height: number },
    x: number,
    y: number,
  ) => void,
): Promise<void> {
  const canvas = ctx.canvas;
  if (frame.keyframe && (canvas.width !== frame.width || canvas.height !== frame.height)) {
    canvas.width = frame.width;
    canvas.height = frame.height;
  }

  for (const t of frame.tiles) {
    const bmp = await decode(t.png);
    drawBitmap(ctx, bmp, t.tx * frame.tile, t.ty * frame.tile);
    bmp.close?.();
  }
}

/** Aggregate stats useful for the session telemetry overlay. */
export function summarizeDeltaFrame(frame: DeltaFrame): {
  keyframe: boolean;
  tilesUpdated: number;
  bytes: number;
} {
  return {
    keyframe: frame.keyframe,
    tilesUpdated: frame.tiles.length,
    bytes: frame.tiles.reduce((acc, t) => acc + t.png.length, 0),
  };
}
