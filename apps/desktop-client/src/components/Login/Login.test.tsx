import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Login } from "./Login";
import { apiService } from "../../services/api";

describe("Login", () => {
  it("renders the login form with branding", () => {
    render(<Login onLoginSuccess={vi.fn()} />);
    expect(screen.getByText("TTGTiSO-Desk")).toBeInTheDocument();
    expect(screen.getByLabelText(/username/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /establish secure session/i }),
    ).toBeInTheDocument();
  });

  it("calls onLoginSuccess after a successful login", async () => {
    const user = userEvent.setup();
    const onLoginSuccess = vi.fn();
    vi.spyOn(apiService, "login").mockResolvedValue({
      success: true,
      message: "ok",
      user: { username: "admin", role: "admin" },
    });

    render(<Login onLoginSuccess={onLoginSuccess} />);
    await user.type(screen.getByLabelText(/password/i), "secret");
    await user.click(
      screen.getByRole("button", { name: /establish secure session/i }),
    );

    await waitFor(() => {
      expect(onLoginSuccess).toHaveBeenCalledWith("", 0, "admin", "admin", "");
    });
  });

  it("shows the server error message when authentication fails", async () => {
    const user = userEvent.setup();
    vi.spyOn(apiService, "login").mockResolvedValue({
      success: false,
      message: "Invalid credentials",
    });

    render(<Login onLoginSuccess={vi.fn()} />);
    await user.type(screen.getByLabelText(/password/i), "wrong");
    await user.click(
      screen.getByRole("button", { name: /establish secure session/i }),
    );

    expect(await screen.findByText("Invalid credentials")).toBeInTheDocument();
  });

  it("shows a connectivity error when the API is unreachable", async () => {
    const user = userEvent.setup();
    vi.spyOn(apiService, "login").mockRejectedValue(new Error("network"));
    vi.spyOn(console, "error").mockImplementation(() => {});

    render(<Login onLoginSuccess={vi.fn()} />);
    await user.type(screen.getByLabelText(/password/i), "secret");
    await user.click(
      screen.getByRole("button", { name: /establish secure session/i }),
    );

    expect(
      await screen.findByText(/cannot connect to api server/i),
    ).toBeInTheDocument();
  });
});
