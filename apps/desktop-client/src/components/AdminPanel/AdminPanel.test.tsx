import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { AdminPanel } from "./AdminPanel";
import { apiService, ActiveSession } from "../../services/api";

const sessions: ActiveSession[] = [
  {
    id: "sess-1",
    username: "operator",
    display_id: 10,
    start_time: "2026-06-10T09:00:00Z",
    cpu_usage: 12.5,
    mem_usage: 30.2,
    host_ip: "10.0.0.5",
  },
];

describe("AdminPanel", () => {
  it("lists active sessions from the API", async () => {
    vi.spyOn(apiService, "getActiveSessions").mockResolvedValue(sessions);
    render(<AdminPanel />);

    expect(await screen.findByText("operator")).toBeInTheDocument();
  });

  it("shows an error state when sessions cannot be loaded", async () => {
    vi.spyOn(apiService, "getActiveSessions").mockRejectedValue(
      new Error("down"),
    );
    vi.spyOn(console, "error").mockImplementation(() => {});
    render(<AdminPanel />);

    expect(
      await screen.findByText(/failed to load sessions/i),
    ).toBeInTheDocument();
  });
});
