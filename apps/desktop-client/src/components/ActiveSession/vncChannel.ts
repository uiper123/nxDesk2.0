export interface ChannelMetrics {
  bitrateKbps: number;
  updatesPerSec: number;
  totalMegabytes: number;
}

export interface InstrumentedChannel {
  socket: WebSocket;
  sample: () => ChannelMetrics;
}

interface CounterState {
  bytesWindow: number;
  messagesWindow: number;
  totalBytes: number;
  windowStartedAt: number;
}

export function createInstrumentedChannel(wsUrl: string, protocols: string[]): InstrumentedChannel {
  const socket = new WebSocket(wsUrl, protocols);
  socket.binaryType = "arraybuffer";

  const counters: CounterState = {
    bytesWindow: 0,
    messagesWindow: 0,
    totalBytes: 0,
    windowStartedAt: performance.now(),
  };

  socket.addEventListener("message", (event: MessageEvent) => {
    const size =
      event.data instanceof ArrayBuffer
        ? event.data.byteLength
        : typeof event.data === "string"
          ? event.data.length
          : (event.data as Blob)?.size ?? 0;

    counters.bytesWindow += size;
    counters.totalBytes += size;
    if (size > 64) {
      counters.messagesWindow += 1;
    }
  });

  const sample = (): ChannelMetrics => {
    const now = performance.now();
    const elapsedSec = Math.max((now - counters.windowStartedAt) / 1000, 0.001);

    const metrics: ChannelMetrics = {
      bitrateKbps: Math.round((counters.bytesWindow * 8) / 1000 / elapsedSec),
      updatesPerSec: Math.round(counters.messagesWindow / elapsedSec),
      totalMegabytes: counters.totalBytes / (1024 * 1024),
    };

    counters.bytesWindow = 0;
    counters.messagesWindow = 0;
    counters.windowStartedAt = now;

    return metrics;
  };

  return { socket, sample };
}

export function describeBitrate(bitrateKbps: number): string {
  if (bitrateKbps >= 1000) {
    return `${(bitrateKbps / 1000).toFixed(1)} Mbps`;
  }
  return `${bitrateKbps} kbps`;
}

export function classifyLinkQuality(params: {
  bitrateKbps: number;
  latencyMs: number;
  connected: boolean;
}): { label: string; bars: 0 | 1 | 2 | 3 } {
  const { bitrateKbps, latencyMs, connected } = params;

  if (!connected) {
    return { label: "Offline", bars: 0 };
  }

  if (latencyMs > 0 && latencyMs < 40 && bitrateKbps > 2000) {
    return { label: "Excellent", bars: 3 };
  }

  if ((latencyMs > 0 && latencyMs < 120) || bitrateKbps > 500) {
    return { label: "Good", bars: 2 };
  }

  return { label: "Degraded", bars: 1 };
}
