type LogLevel = "debug" | "info" | "warn" | "error";

interface ClientLogEntry {
  timestamp: string;
  level: LogLevel;
  scope: string;
  message: string;
  detail?: unknown;
}

const MAX_BUFFER = 500;
const buffer: ClientLogEntry[] = [];

function record(level: LogLevel, scope: string, message: string, detail?: unknown): void {
  const entry: ClientLogEntry = {
    timestamp: new Date().toISOString(),
    level,
    scope,
    message,
    detail,
  };
  buffer.push(entry);
  if (buffer.length > MAX_BUFFER) {
    buffer.shift();
  }
  if (import.meta.env.DEV) {
    const line = `[${entry.timestamp}] [${scope}] ${message}`;
    if (level === "error") console.error(line, detail ?? "");
    else if (level === "warn") console.warn(line, detail ?? "");
    else console.info(line, detail ?? "");
  }
}

export const logger = {
  debug: (scope: string, message: string, detail?: unknown) => record("debug", scope, message, detail),
  info: (scope: string, message: string, detail?: unknown) => record("info", scope, message, detail),
  warn: (scope: string, message: string, detail?: unknown) => record("warn", scope, message, detail),
  error: (scope: string, message: string, detail?: unknown) => record("error", scope, message, detail),
  history: (): readonly ClientLogEntry[] => buffer,
};
