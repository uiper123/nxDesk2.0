import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Settings } from "./Settings";
import { apiService } from "../../services/api";

const savedSettings = {
  quality: "high",
  encoder: "vaapi",
  fps: 60,
  audio: true,
};

describe("Settings", () => {
  it("loads and displays persisted settings", async () => {
    vi.spyOn(apiService, "getSettings").mockResolvedValue(savedSettings);
    render(<Settings />);

    await waitFor(() => {
      expect(screen.getByDisplayValue(/high \(1080p/i)).toBeInTheDocument();
    });
    expect(screen.getByDisplayValue("60 FPS")).toBeInTheDocument();
    expect(screen.getByRole("checkbox")).toBeChecked();
  });

  it("saves updated settings through the API", async () => {
    const user = userEvent.setup();
    vi.spyOn(apiService, "getSettings").mockResolvedValue(savedSettings);
    const updateSpy = vi
      .spyOn(apiService, "updateSettings")
      .mockResolvedValue({ success: true, message: "ok" });
    vi.spyOn(window, "alert").mockImplementation(() => {});

    render(<Settings />);
    await screen.findByDisplayValue("60 FPS");

    await user.selectOptions(screen.getByDisplayValue("60 FPS"), "30");
    await user.click(screen.getByRole("button", { name: /save changes/i }));

    await waitFor(() => {
      expect(updateSpy).toHaveBeenCalledWith({
        quality: "high",
        encoder: "vaapi",
        fps: 30,
        audio: true,
      });
    });
  });

  it("shows an error state when settings cannot be loaded", async () => {
    vi.spyOn(apiService, "getSettings").mockRejectedValue(new Error("down"));
    vi.spyOn(console, "error").mockImplementation(() => {});
    render(<Settings />);

    expect(
      await screen.findByText(/failed to load settings/i),
    ).toBeInTheDocument();
  });
});
