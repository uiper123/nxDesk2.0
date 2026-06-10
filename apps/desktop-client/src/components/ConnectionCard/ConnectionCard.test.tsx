import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { ConnectionCard } from "./ConnectionCard";

describe("ConnectionCard", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
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

  it("invokes onConnected after all steps complete", () => {
    const onConnected = vi.fn();
    render(
      <ConnectionCard
        host="10.0.0.5"
        username="operator"
        onConnected={onConnected}
        onCancel={vi.fn()}
      />,
    );

    for (let i = 0; i < 8; i++) {
      act(() => {
        vi.advanceTimersByTime(1500);
      });
    }

    expect(onConnected).toHaveBeenCalledTimes(1);
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
