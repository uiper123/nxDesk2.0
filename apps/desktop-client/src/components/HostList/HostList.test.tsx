import { describe, it, expect, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders as render } from "../../test/render";
import { HostList } from "./HostList";
import { apiService, Host } from "../../services/api";

const hosts: Host[] = [
  {
    id: "h1",
    name: "Astra Workstation 01",
    ip: "10.0.0.5",
    port: 2222,
    status: "online",
    active_sessions: 1,
    operating_system: "Astra Linux 1.8",
  },
  {
    id: "h2",
    name: "Astra Workstation 02",
    ip: "10.0.0.6",
    port: 2222,
    status: "offline",
    active_sessions: 0,
    operating_system: "Astra Linux 1.8",
  },
];

describe("HostList", () => {
  it("renders hosts returned by the API", async () => {
    vi.spyOn(apiService, "getHosts").mockResolvedValue(hosts);
    render(<HostList onSelectHost={vi.fn()} />);

    expect(await screen.findByText("Astra Workstation 01")).toBeInTheDocument();
    expect(screen.getByText("Astra Workstation 02")).toBeInTheDocument();
  });

  it("shows an error state when the host registry is unreachable", async () => {
    vi.spyOn(apiService, "getHosts").mockRejectedValue(new Error("down"));
    vi.spyOn(console, "error").mockImplementation(() => {});
    render(<HostList onSelectHost={vi.fn()} />);

    expect(await screen.findByText(/failed to load hosts/i)).toBeInTheDocument();
  });

  it("filters hosts via the search input", async () => {
    vi.spyOn(apiService, "getHosts").mockResolvedValue(hosts);
    const user = userEvent.setup();

    render(<HostList onSelectHost={vi.fn()} />);
    expect(await screen.findByText("Astra Workstation 01")).toBeInTheDocument();

    const search = screen.getByPlaceholderText(/поиск/i);
    await user.type(search, "10.0.0.6");

    await waitFor(() => {
      expect(screen.queryByText("Astra Workstation 01")).not.toBeInTheDocument();
    });
    expect(screen.getByText("Astra Workstation 02")).toBeInTheDocument();
  });
});
