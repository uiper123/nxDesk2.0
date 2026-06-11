export type ConnectionStatus = "connecting" | "connected" | "disconnected" | "error";
export type ConnectionMode = "performance" | "balanced" | "clarity";

export interface ConnectionModeDetails {
  label: string;
  description: string;
  badge: string;
  recommendedScale: number | "fit";
  showChrome: "compact" | "standard" | "detailed";
}

export interface ConnectionHealth {
  tone: "good" | "warn" | "danger";
  title: string;
  detail: string;
}

const MODE_DETAILS: Record<ConnectionMode, ConnectionModeDetails> = {
  performance: {
    label: "Performance",
    description: "Минимум лишнего UI и авто-fit для слабых каналов.",
    badge: "low-latency",
    recommendedScale: "fit",
    showChrome: "compact",
  },
  balanced: {
    label: "Balanced",
    description: "Комфортный режим для большинства удалённых сессий.",
    badge: "default",
    recommendedScale: 90,
    showChrome: "standard",
  },
  clarity: {
    label: "Clarity",
    description: "Больше пространства, 100% масштаб и максимальная читаемость.",
    badge: "hi-fi",
    recommendedScale: 100,
    showChrome: "detailed",
  },
};

export function getConnectionModeDetails(mode: ConnectionMode): ConnectionModeDetails {
  return MODE_DETAILS[mode];
}

export function buildRemoteDesktopUrls(
  apiBaseUrl: string,
  host: string,
  displayId: number,
  monitorIndex = 0,
) {
  const base = apiBaseUrl.replace(/\/$/, "");
  const wsBase = base.replace(/^https:/, "wss:").replace(/^http:/, "ws:");
  const query = new URLSearchParams({ host, display: String(displayId) });
  if (monitorIndex > 0) {
    query.set("monitor", String(monitorIndex));
  }

  return {
    wsUrl: `${wsBase}/ws/vnc?${query.toString()}`,
    uploadUrl: `${base}/upload`,
  };
}

export interface RemoteMonitor {
  index: number;
  name: string;
  width: number;
  height: number;
  x: number;
  y: number;
  isPrimary: boolean;
}

/**
 * Parse the agent's `monitors` control-command JSON reply into typed
 * RemoteMonitor records. Tolerant of missing fields so a partial reply still
 * yields a usable (sorted, primary-first) list.
 */
export function parseMonitorList(raw: unknown): RemoteMonitor[] {
  const obj = raw as { monitors?: unknown } | null;
  const list = Array.isArray(obj?.monitors) ? (obj!.monitors as unknown[]) : [];
  const monitors: RemoteMonitor[] = list.map((m, i) => {
    const r = (m ?? {}) as Record<string, unknown>;
    const num = (v: unknown, fallback: number) =>
      typeof v === "number" && Number.isFinite(v) ? v : fallback;
    return {
      index: num(r.index, i),
      name: typeof r.name === "string" && r.name ? r.name : `Monitor ${i + 1}`,
      width: num(r.width, 0),
      height: num(r.height, 0),
      x: num(r.x, 0),
      y: num(r.y, 0),
      isPrimary: r.is_primary === true || r.isPrimary === true,
    };
  });
  monitors.sort((a, b) => Number(b.isPrimary) - Number(a.isPrimary) || a.index - b.index);
  return monitors;
}

export function describeMonitor(monitor: RemoteMonitor): string {
  const res = monitor.width > 0 && monitor.height > 0 ? ` · ${monitor.width}×${monitor.height}` : "";
  const primary = monitor.isPrimary ? " (основной)" : "";
  return `${monitor.name}${res}${primary}`;
}

export type AccessMode = "unattended" | "ask-user";

export interface AccessModeDetails {
  label: string;
  description: string;
  tone: "good" | "warn";
}

const ACCESS_MODE_DETAILS: Record<AccessMode, AccessModeDetails> = {
  unattended: {
    label: "Постоянный доступ",
    description: "Подключение без подтверждения на стороне хоста (unattended).",
    tone: "warn",
  },
  "ask-user": {
    label: "Спросить пользователя",
    description: "Локальный пользователь должен подтвердить входящее подключение.",
    tone: "good",
  },
};

export function getAccessModeDetails(mode: AccessMode): AccessModeDetails {
  return ACCESS_MODE_DETAILS[mode];
}

export function buildUploadUrl(apiBaseUrl: string, filename: string): string {
  return `${apiBaseUrl.replace(/\/$/, "")}/upload/${encodeURIComponent(filename)}`;
}

export function formatDuration(totalSeconds: number): string {
  const safeSeconds = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(safeSeconds / 3600);
  const minutes = Math.floor((safeSeconds % 3600) / 60);
  const seconds = safeSeconds % 60;

  return [hours, minutes, seconds].map((value) => String(value).padStart(2, "0")).join(":");
}

export function formatHostEndpoint(host: string, port: number, displayId?: number): string {
  return `${host}:${port} · display :${displayId ?? 0}`;
}

export function classifyConnectionHealth(params: {
  status: ConnectionStatus;
  retryCount: number;
  sessionSeconds: number;
  clipboardSynced: boolean;
}): ConnectionHealth {
  const { status, retryCount, sessionSeconds, clipboardSynced } = params;

  if (status === "error") {
    return {
      tone: "danger",
      title: "Сеанс потерян",
      detail: "Канал прерван. Нажмите Reconnect, чтобы поднять VNC-туннель заново.",
    };
  }

  if (status === "connecting") {
    return {
      tone: "warn",
      title: "Установка соединения",
      detail: "Поднимаем WebSocket-туннель и согласуем VNC-протокол с агентом…",
    };
  }

  if (status === "disconnected") {
    return {
      tone: "warn",
      title: "Соединение остановлено",
      detail: "Сессия закрыта или недоступна. Можно повторить подключение без выхода из экрана.",
    };
  }

  if (retryCount > 0) {
    return {
      tone: "warn",
      title: "Сессия восстановлена",
      detail: `Подключение пережило ${retryCount} повтор${retryCount === 1 ? "" : "а"}; канал стабилизирован на ${formatDuration(sessionSeconds)}.`,
    };
  }

  return {
    tone: clipboardSynced ? "good" : "warn",
    title: "Защищённый канал активен",
    detail: `Удалённый экран открыт ${formatDuration(sessionSeconds)}. ${clipboardSynced ? "Буфер обмена синхронизирован." : "Буфер обмена ждёт первой синхронизации."}`,
  };
}
