import { describe, it, expect } from "vitest";
import {
  buildRemoteDesktopUrls,
  buildUploadUrl,
  classifyConnectionHealth,
  formatDuration,
  formatHostEndpoint,
  getConnectionModeDetails,
} from "./remoteAccess";

describe("buildRemoteDesktopUrls", () => {
  it("converts http API base to ws URL with host and display params", () => {
    const { wsUrl } = buildRemoteDesktopUrls("http://127.0.0.1:3001/api", "10.0.0.5", 10);
    expect(wsUrl).toBe("ws://127.0.0.1:3001/api/ws/vnc?host=10.0.0.5&display=10");
  });

  it("converts https API base to wss URL", () => {
    const { wsUrl } = buildRemoteDesktopUrls("https://relay.local/api/", "192.168.1.20", 0);
    expect(wsUrl).toBe("wss://relay.local/api/ws/vnc?host=192.168.1.20&display=0");
  });

  it("URL-encodes unusual host values", () => {
    const { wsUrl } = buildRemoteDesktopUrls("http://127.0.0.1:3001/api", "host name", 1);
    expect(wsUrl).toContain("host=host+name");
  });
});

describe("buildUploadUrl", () => {
  it("builds upload endpoint with encoded filename", () => {
    expect(buildUploadUrl("http://127.0.0.1:3001/api", "отчёт 2026.pdf")).toBe(
      "http://127.0.0.1:3001/api/upload/" + encodeURIComponent("отчёт 2026.pdf"),
    );
  });

  it("strips trailing slash from base", () => {
    expect(buildUploadUrl("http://127.0.0.1:3001/api/", "a.txt")).toBe(
      "http://127.0.0.1:3001/api/upload/a.txt",
    );
  });
});

describe("formatDuration", () => {
  it("formats zero as 00:00:00", () => {
    expect(formatDuration(0)).toBe("00:00:00");
  });

  it("formats hours, minutes and seconds", () => {
    expect(formatDuration(3661)).toBe("01:01:01");
  });

  it("clamps negative values to zero", () => {
    expect(formatDuration(-5)).toBe("00:00:00");
  });
});

describe("formatHostEndpoint", () => {
  it("includes host, port and display", () => {
    expect(formatHostEndpoint("10.0.0.5", 2222, 10)).toBe("10.0.0.5:2222 · display :10");
  });

  it("defaults display to 0", () => {
    expect(formatHostEndpoint("10.0.0.5", 2222)).toBe("10.0.0.5:2222 · display :0");
  });
});

describe("getConnectionModeDetails", () => {
  it("returns distinct details for each mode", () => {
    const labels = (["performance", "balanced", "clarity"] as const).map(
      (m) => getConnectionModeDetails(m).label,
    );
    expect(new Set(labels).size).toBe(3);
  });

  it("recommends auto-fit for performance mode", () => {
    expect(getConnectionModeDetails("performance").recommendedScale).toBe("fit");
  });
});

describe("classifyConnectionHealth", () => {
  it("marks error status as danger", () => {
    const health = classifyConnectionHealth({
      status: "error",
      retryCount: 0,
      sessionSeconds: 0,
      clipboardSynced: false,
    });
    expect(health.tone).toBe("danger");
  });

  it("marks connecting status as warn", () => {
    const health = classifyConnectionHealth({
      status: "connecting",
      retryCount: 0,
      sessionSeconds: 0,
      clipboardSynced: false,
    });
    expect(health.tone).toBe("warn");
    expect(health.title).toMatch(/соединени/i);
  });

  it("reports recovered session after retries", () => {
    const health = classifyConnectionHealth({
      status: "connected",
      retryCount: 2,
      sessionSeconds: 90,
      clipboardSynced: true,
    });
    expect(health.tone).toBe("warn");
    expect(health.detail).toContain("00:01:30");
  });

  it("reports good tone for stable synced session", () => {
    const health = classifyConnectionHealth({
      status: "connected",
      retryCount: 0,
      sessionSeconds: 10,
      clipboardSynced: true,
    });
    expect(health.tone).toBe("good");
  });
});
