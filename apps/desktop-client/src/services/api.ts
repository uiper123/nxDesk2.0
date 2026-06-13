import { invoke } from "@tauri-apps/api/core";

export const API_BASE_URL: string =
  (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://127.0.0.1:3001/api';

const DEFAULT_TIMEOUT_MS = 15_000;

export class ApiError extends Error {
  readonly status: number;
  readonly statusText: string;
  readonly body?: unknown;

  constructor(status: number, statusText: string, body?: unknown) {
    const detail =
      body && typeof body === 'object' && 'message' in (body as Record<string, unknown>)
        ? String((body as Record<string, unknown>).message)
        : statusText;
    super(`API ${status}: ${detail}`);
    this.name = 'ApiError';
    this.status = status;
    this.statusText = statusText;
    this.body = body;
  }
}

export class ApiTimeoutError extends Error {
  constructor(endpoint: string, timeoutMs: number) {
    super(`Request to ${endpoint} timed out after ${timeoutMs}ms`);
    this.name = 'ApiTimeoutError';
  }
}

export interface LoginRequest {
  host: string;
  port: number;
  username: string;
  password: string;
}

export interface LoginResponse {
  success: boolean;
  message: string;
  user?: {
    username: string;
    role: string;
    token?: string;
  };
}

export interface Host {
  id: string;
  name: string;
  ip: string;
  port: number;
  status: 'online' | 'offline' | 'busy';
  active_sessions: number;
  operating_system: string;
  ssh_public_key?: string;
  ssh_public_key_path?: string;
  ssh_private_key_path?: string;
}

export interface ActiveSession {
  id: string;
  username: string;
  display_id: number;
  start_time: string;
  cpu_usage: number;
  mem_usage: number;
  host_ip: string;
}

export interface LogEntry {
  timestamp: string;
  level: 'INFO' | 'WARN' | 'ERROR' | 'AUDIT';
  message: string;
}

export interface Settings {
  quality: string;
  encoder: string;
  fps: number;
  audio: boolean;
}

export interface AppInfo {
  name: string;
  exec: string;
}

export interface AppsResponse {
  applications: AppInfo[];
  count: number;
}

export interface SshIdentityInfo {
  public_key: string;
  public_key_path: string;
  private_key_path: string;
}

class ApiService {
  private async request<T>(
    endpoint: string,
    options?: RequestInit,
    timeoutMs: number = DEFAULT_TIMEOUT_MS
  ): Promise<T> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);

    let response: Response;
    try {
      response = await fetch(`${API_BASE_URL}${endpoint}`, {
        ...options,
        signal: controller.signal,
        headers: {
          'Content-Type': 'application/json',
          ...options?.headers,
        },
      });
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') {
        throw new ApiTimeoutError(endpoint, timeoutMs);
      }
      throw err;
    } finally {
      clearTimeout(timer);
    }

    if (!response.ok) {
      let body: unknown;
      try {
        body = await response.json();
      } catch {
        body = undefined;
      }
      throw new ApiError(response.status, response.statusText, body);
    }

    return response.json() as Promise<T>;
  }

  async login(credentials: LoginRequest): Promise<LoginResponse> {
    return this.request<LoginResponse>('/auth/login', {
      method: 'POST',
      body: JSON.stringify(credentials),
    });
  }

  async startSession(credentials: LoginRequest): Promise<LoginResponse> {
    return this.request<LoginResponse>('/sessions/start', {
      method: 'POST',
      body: JSON.stringify(credentials),
    });
  }

  async addHost(host: {
    name: string;
    ip: string;
    port: number;
    ssh_public_key?: string;
    ssh_public_key_path?: string;
    ssh_private_key_path?: string;
  }): Promise<{ success: boolean; message: string }> {
    return this.request('/hosts', {
      method: 'POST',
      body: JSON.stringify(host),
    });
  }

  async updateHost(
    hostIp: string,
    host: {
      name: string;
      ip: string;
      port: number;
      ssh_public_key?: string;
      ssh_public_key_path?: string;
      ssh_private_key_path?: string;
    }
  ): Promise<{ success: boolean; message: string }> {
    return this.request(`/hosts/${encodeURIComponent(hostIp)}`, {
      method: 'POST',
      body: JSON.stringify(host),
    });
  }

  async deleteHost(hostIp: string): Promise<{ success: boolean; message: string }> {
    return this.request(`/hosts/${encodeURIComponent(hostIp)}`, {
      method: 'DELETE',
    });
  }

  async getHosts(): Promise<Host[]> {
    return this.request<Host[]>('/hosts');
  }

  async getDiscoveredHosts(): Promise<Host[]> {
    return this.request<Host[]>('/hosts/discovered');
  }

  async getActiveSessions(): Promise<ActiveSession[]> {
    return this.request<ActiveSession[]>('/sessions/active');
  }

  async getSystemUsers(ip: string): Promise<string[]> {
    return this.request<string[]>(`/hosts/${encodeURIComponent(ip)}/users`);
  }

  async terminateSession(sessionId: string): Promise<{ success: boolean; message: string }> {
    return this.request(`/sessions/${sessionId}/terminate`, {
      method: 'POST',
    });
  }

  async getLogs(): Promise<LogEntry[]> {
    return this.request<LogEntry[]>('/logs');
  }

  async getSettings(): Promise<Settings> {
    return this.request<Settings>('/settings');
  }

  async updateSettings(settings: Settings): Promise<{ success: boolean; message: string }> {
    return this.request('/settings', {
      method: 'POST',
      body: JSON.stringify(settings),
    });
  }

  async healthCheck(): Promise<{ ok: boolean; latencyMs: number }> {
    const started = performance.now();
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 5_000);
    try {
      const response = await fetch(`${API_BASE_URL}/health`, { signal: controller.signal });
      return { ok: response.ok, latencyMs: Math.round(performance.now() - started) };
    } catch {
      return { ok: false, latencyMs: Math.round(performance.now() - started) };
    } finally {
      clearTimeout(timer);
    }
  }

  async getApplications(hostIp: string): Promise<AppsResponse> {
    return this.request<AppsResponse>(`/hosts/${encodeURIComponent(hostIp)}/applications`);
  }

  async launchApplication(sessionId: string, command: string): Promise<{ success: boolean; message: string }> {
    return this.request(`/sessions/${sessionId}/launch`, {
      method: 'POST',
      body: JSON.stringify({ command }),
    });
  }

  async getHostMetrics(hostIp: string): Promise<SystemMetrics> {
    return this.request<SystemMetrics>(`/hosts/${encodeURIComponent(hostIp)}/metrics`);
  }

  async executePowerAction(hostIp: string, action: 'reboot' | 'shutdown' | 'lock'): Promise<{ success: boolean; message: string }> {
    return this.request(`/hosts/${encodeURIComponent(hostIp)}/power`, {
      method: 'POST',
      body: JSON.stringify({ action }),
    });
  }

  async ensureSshIdentity(): Promise<SshIdentityInfo> {
    return invoke<SshIdentityInfo>('ensure_ssh_identity');
  }

  async regenerateSshIdentity(): Promise<SshIdentityInfo> {
    return invoke<SshIdentityInfo>('regenerate_ssh_identity');
  }
}

export interface SystemMetrics {
  hostname: string;
  os: string;
  uptime_seconds: number;
  cpu_count: number;
  load_average_1m: number;
  load_average_5m: number;
  load_average_15m: number;
  memory_total_mb: number;
  memory_available_mb: number;
  memory_used_mb: number;
  memory_usage_percent: number;
}

export const apiService = new ApiService();
