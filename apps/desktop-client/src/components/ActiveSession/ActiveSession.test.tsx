import { describe, it, expect, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProviders as render } from "../../test/render";

vi.mock("@novnc/novnc", () => {
  class MockRFB {
    scaleViewport = false;
    resizeSession = false;
    addEventListener = vi.fn();
    removeEventListener = vi.fn();
    disconnect = vi.fn();
  }
  return { default: MockRFB };
});

import { ActiveSession } from "./ActiveSession";

describe("ActiveSession", () => {
  it("renders the session toolbar with host identity", () => {
    render(
      <ActiveSession
        host="10.0.0.5"
        port={2222}
        username="operator"
        displayId={10}
        onDisconnect={vi.fn()}
      />,
    );

    expect(screen.getAllByText(/operator/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/10\.0\.0\.5/).length).toBeGreaterThan(0);
  });

  it("invokes onDisconnect when disconnect is clicked", () => {
    const onDisconnect = vi.fn();
    render(
      <ActiveSession
        host="10.0.0.5"
        port={2222}
        username="operator"
        displayId={10}
        onDisconnect={onDisconnect}
      />,
    );

    const btn = screen.getByRole("button", { name: /disconnect|завершить/i });
    btn.click();
    expect(onDisconnect).toHaveBeenCalledTimes(1);
  });
});
