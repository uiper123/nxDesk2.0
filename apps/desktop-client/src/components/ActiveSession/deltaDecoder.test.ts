import { describe, expect, it } from "vitest";
import {
  DELTA_MAGIC,
  FLAG_KEYFRAME,
  isDeltaFrame,
  parseDeltaFrame,
  summarizeDeltaFrame,
} from "./deltaDecoder";

/** Build a delta payload matching the Rust wire format, for round-trip tests. */
function buildPayload(opts: {
  keyframe: boolean;
  width: number;
  height: number;
  tile: number;
  tiles: { tx: number; ty: number; png: number[] }[];
}): Uint8Array {
  const parts: number[] = [];
  parts.push(DELTA_MAGIC);
  parts.push(opts.keyframe ? FLAG_KEYFRAME : 0);
  parts.push((opts.width >> 8) & 0xff, opts.width & 0xff);
  parts.push((opts.height >> 8) & 0xff, opts.height & 0xff);
  parts.push((opts.tile >> 8) & 0xff, opts.tile & 0xff);
  parts.push((opts.tiles.length >> 8) & 0xff, opts.tiles.length & 0xff);
  for (const t of opts.tiles) {
    parts.push((t.tx >> 8) & 0xff, t.tx & 0xff);
    parts.push((t.ty >> 8) & 0xff, t.ty & 0xff);
    const len = t.png.length;
    parts.push((len >>> 24) & 0xff, (len >>> 16) & 0xff, (len >>> 8) & 0xff, len & 0xff);
    parts.push(...t.png);
  }
  return new Uint8Array(parts);
}

describe("deltaDecoder", () => {
  it("recognizes a delta frame by magic byte", () => {
    expect(isDeltaFrame(new Uint8Array([DELTA_MAGIC, 0, 0, 0, 0, 0, 0, 0, 0, 0]))).toBe(true);
    expect(isDeltaFrame(new Uint8Array([0x00, 0x01]))).toBe(false);
    // RFB/VNC and PNG streams must not be mistaken for delta frames.
    expect(isDeltaFrame(new Uint8Array([0x89, 0x50, 0x4e, 0x47]))).toBe(false);
  });

  it("parses a keyframe with two tiles", () => {
    const payload = buildPayload({
      keyframe: true,
      width: 128,
      height: 64,
      tile: 64,
      tiles: [
        { tx: 0, ty: 0, png: [1, 2, 3] },
        { tx: 1, ty: 0, png: [4, 5] },
      ],
    });
    const frame = parseDeltaFrame(payload);
    expect(frame.keyframe).toBe(true);
    expect(frame.width).toBe(128);
    expect(frame.height).toBe(64);
    expect(frame.tile).toBe(64);
    expect(frame.tiles).toHaveLength(2);
    expect(Array.from(frame.tiles[0].png)).toEqual([1, 2, 3]);
    expect(frame.tiles[1].tx).toBe(1);
  });

  it("parses a delta (non-keyframe) with zero tiles", () => {
    const payload = buildPayload({
      keyframe: false,
      width: 1920,
      height: 1080,
      tile: 64,
      tiles: [],
    });
    const frame = parseDeltaFrame(payload);
    expect(frame.keyframe).toBe(false);
    expect(frame.tiles).toHaveLength(0);
  });

  it("throws on a record that overruns the buffer", () => {
    // Claim one tile of length 100 but provide no payload bytes.
    const payload = new Uint8Array([
      DELTA_MAGIC, 0, 0, 64, 0, 64, 0, 64, 0, 1, // header + count=1
      0, 0, 0, 0, 0, 0, 0, 100, // tx=0 ty=0 len=100 (but no data follows)
    ]);
    expect(() => parseDeltaFrame(payload)).toThrow(/overruns/);
  });

  it("summarizes updated tiles and byte totals", () => {
    const payload = buildPayload({
      keyframe: false,
      width: 128,
      height: 64,
      tile: 64,
      tiles: [
        { tx: 0, ty: 0, png: [1, 2, 3, 4] },
        { tx: 1, ty: 0, png: [5, 6] },
      ],
    });
    const summary = summarizeDeltaFrame(parseDeltaFrame(payload));
    expect(summary.tilesUpdated).toBe(2);
    expect(summary.bytes).toBe(6);
    expect(summary.keyframe).toBe(false);
  });
});
