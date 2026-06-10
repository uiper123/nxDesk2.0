import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { ConnectionCard } from "./ConnectionCard";
import { apiService } from "../../services/api";

vi.mock("../../services/api", () => ({
  apiService: {
    healthCheck: vi.fn(),
    getHosts: vi.fn(),
    getActiveSessions: vi.fn(),
  },
}));

const mockedApi = vi.mocked(apiService);

describe("ConnectionCard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedApi.healthCheck.mockResolvedValue({ ok: true, latencyMs: 12 });
    mockedApi.getHosts.mockResolvedValue([
      {
        id: "1",
        name: "astra-01",
        ip: "10.0.0.5",
        port: 2222,
        status: "online",
        operating_system: "Astra Linux 1.7",
        active_sessions: 0,
      } as any,
    ]);
    mockedApi.getActiveSessions.mockResolvedValue([]);
  });

  it("renders the connection target", () => {
    render(
      <ConnectionCard
        host="10.0.0.5"
        username="operator"
        onConnected={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    expect(screen.getByText("operator@10.0.0.5")).toBeInTheDocument();
    expect(screen.getByText(/establishing connection/i)).toBeInTheDocument();
  });

  it("invokes onConnected after successful pre-flight checks", async () => {
    const onConnected = vi.fn();
    render(
      <ConnectionCard
        host="10.0.0.5"
        username="operator"
        onConnected={onConnected}
        onCancel={vi.fn()}
      />,
    );

    await waitFor(() => expect(onConnected).toHaveBeenCalledTimes(1), { timeout: 6000 });
    expect(mockedApi.healthCheck).toHaveBeenCalled();
    expect(mockedApi.getHosts).toHaveBeenCalled();
  });

  it("shows a failure state when the API is unreachable", async () => {
    mockedApi.healthCheck.mockResolvedValue({ ok: false, latencyMs: 5000 });
    const onConnected = vi.fn();
    render(
      <ConnectionCard
        host="10.0.0.5"
        username="operator"
        onConnected={onConnected}
        onCancel={vi.fn()}
      />,
    );

    await waitFor(() => expect(screen.getByText(/сбой подключения/i)).toBeInTheDocument(), {
      timeout: 6000,
    });
    expect(onConnected).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /повторить проверку/i })).toBeInTheDocument();
  });

  it("invokes onCancel when the cancel button is clicked", () => {
    const onCancel = vi.fn();
    render(
      <ConnectionCard
        host="10.0.0.5"
        username="operator"
        onConnected={vi.fn()}
        onCancel={onCancel}
      />,
    );

    screen.getByRole("button", { name: /cancel connection/i }).click();
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});
