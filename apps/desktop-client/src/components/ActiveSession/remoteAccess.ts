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

export function buildRemoteDesktopUrls(apiBaseUrl: string, host: string, displayId: number) {
  const base = apiBaseUrl.replace(/\/$/, "");
  const wsBase = base.replace(/^https:/, "wss:").replace(/^http:/, "ws:");
  const query = new URLSearchParams({ host, display: String(displayId) });

  return {
    wsUrl: `${wsBase}/ws/vnc?${query.toString()}`,
    uploadUrl: `${base}/upload`,
  };
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
