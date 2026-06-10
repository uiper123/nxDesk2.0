import { describe, it, expect, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProviders as render } from "../../test/render";
import userEvent from "@testing-library/user-event";
import { Logs } from "./Logs";
import { apiService, LogEntry } from "../../services/api";

const sampleLogs: LogEntry[] = [
  { timestamp: "2026-06-10 10:00:01", level: "INFO", message: "Agent started" },
  { timestamp: "2026-06-10 10:00:02", level: "ERROR", message: "Pipeline crashed" },
  { timestamp: "2026-06-10 10:00:03", level: "AUDIT", message: "User admin logged in" },
];

describe("Logs", () => {
  it("renders fetched log entries", async () => {
    vi.spyOn(apiService, "getLogs").mockResolvedValue(sampleLogs);
    render(<Logs />);

    expect(await screen.findByText("Agent started")).toBeInTheDocument();
    expect(screen.getByText("Pipeline crashed")).toBeInTheDocument();
    expect(screen.getByText("User admin logged in")).toBeInTheDocument();
  });

  it("filters log entries by level", async () => {
    const user = userEvent.setup();
    vi.spyOn(apiService, "getLogs").mockResolvedValue(sampleLogs);
    render(<Logs />);

    await screen.findByText("Agent started");
    await user.selectOptions(screen.getByRole("combobox"), "ERROR");

    expect(screen.getByText("Pipeline crashed")).toBeInTheDocument();
    expect(screen.queryByText("Agent started")).not.toBeInTheDocument();
  });

  it("shows an error state when the API fails", async () => {
    vi.spyOn(apiService, "getLogs").mockRejectedValue(new Error("down"));
    vi.spyOn(console, "error").mockImplementation(() => {});
    render(<Logs />);

    expect(await screen.findByText(/failed to load logs/i)).toBeInTheDocument();
  });
});
