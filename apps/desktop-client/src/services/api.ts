const API_BASE_URL = 'http://127.0.0.1:3001/api';

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

class ApiService {
  private async request<T>(
    endpoint: string,
    options?: RequestInit
  ): Promise<T> {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    });

    if (!response.ok) {
      throw new Error(`API error: ${response.statusText}`);
    }

    return response.json();
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

  async addHost(host: { name: string; ip: string; port: number }): Promise<{ success: boolean; message: string }> {
    return this.request('/hosts', {
      method: 'POST',
      body: JSON.stringify(host),
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
    return this.request<string[]>(`/hosts/${ip}/users`);
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

  async healthCheck(): Promise<string> {
    const response = await fetch(`${API_BASE_URL}/health`);
    return response.text();
  }

  async getApplications(hostIp: string): Promise<AppsResponse> {
    return this.request<AppsResponse>(`/hosts/${hostIp}/applications`);
  }

  async launchApplication(sessionId: string, command: string): Promise<{ success: boolean; message: string }> {
    return this.request(`/sessions/${sessionId}/launch`, {
      method: 'POST',
      body: JSON.stringify({ command }),
    });
  }
}

export const apiService = new ApiService();
