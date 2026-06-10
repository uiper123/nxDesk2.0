import { describe, it, expect, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import { renderWithProviders as render } from "../../test/render";
import { Dashboard } from "./Dashboard";
import { apiService, Host, ActiveSession, LogEntry } from "../../services/api";

const hosts: Host[] = [
  { id: "1", name: "astra-01", ip: "10.0.0.1", port: 2222, status: "online", active_sessions: 2, operating_system: "Astra Linux SE 1.8" },
  { id: "2", name: "astra-02", ip: "10.0.0.2", port: 2222, status: "offline", active_sessions: 0, operating_system: "Astra Linux SE 1.8" },
];

const sessions: ActiveSession[] = [
  { id: "s1", username: "operator", display_id: 10, start_time: "2026-06-10 09:00", cpu_usage: 12, mem_usage: 30, host_ip: "10.0.0.1" },
];

const logs: LogEntry[] = [
  { timestamp: "2026-06-10 09:05", level: "AUDIT", message: "Session started" },
];

describe("Dashboard", () => {
  it("renders stats derived from API data", async () => {
    vi.spyOn(apiService, "getHosts").mockResolvedValue(hosts);
    vi.spyOn(apiService, "getActiveSessions").mockResolvedValue(sessions);
    vi.spyOn(apiService, "getLogs").mockResolvedValue(logs);

    render(<Dashboard onNavigate={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText(/Всего хостов/)).toBeInTheDocument();
    });
    expect(screen.getAllByText(/operator/).length).toBeGreaterThan(0);
    expect(screen.getByText(/Session started/)).toBeInTheDocument();
  });

  it("invokes onNavigate when a stat card is clicked", async () => {
    vi.spyOn(apiService, "getHosts").mockResolvedValue(hosts);
    vi.spyOn(apiService, "getActiveSessions").mockResolvedValue(sessions);
    vi.spyOn(apiService, "getLogs").mockResolvedValue(logs);
    const onNavigate = vi.fn();

    render(<Dashboard onNavigate={onNavigate} />);
    await waitFor(() => {
      expect(screen.getByText(/Всего хостов/)).toBeInTheDocument();
    });

    const buttons = screen.getAllByRole("button");
    buttons[0].click();
    expect(onNavigate).toHaveBeenCalled();
  });
});
