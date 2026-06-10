import { describe, it, expect } from "vitest";
import { classifyLinkQuality, describeBitrate } from "./vncChannel";

describe("describeBitrate", () => {
  it("formats sub-megabit rates in kbps", () => {
    expect(describeBitrate(640)).toBe("640 kbps");
  });

  it("formats megabit rates with one decimal", () => {
    expect(describeBitrate(2400)).toBe("2.4 Mbps");
  });
});

describe("classifyLinkQuality", () => {
  it("returns Offline with zero bars when disconnected", () => {
    expect(classifyLinkQuality({ bitrateKbps: 5000, latencyMs: 10, connected: false })).toEqual({
      label: "Offline",
      bars: 0,
    });
  });

  it("returns Excellent for fast low-latency links", () => {
    expect(classifyLinkQuality({ bitrateKbps: 4000, latencyMs: 12, connected: true })).toEqual({
      label: "Excellent",
      bars: 3,
    });
  });

  it("returns Good for medium links", () => {
    expect(classifyLinkQuality({ bitrateKbps: 800, latencyMs: 0, connected: true })).toEqual({
      label: "Good",
      bars: 2,
    });
  });

  it("returns Degraded for slow links", () => {
    expect(classifyLinkQuality({ bitrateKbps: 100, latencyMs: 0, connected: true })).toEqual({
      label: "Degraded",
      bars: 1,
    });
  });
});
