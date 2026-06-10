import React from "react";
import { render, RenderResult } from "@testing-library/react";
import { ToastProvider } from "../components/Toast";

export function renderWithProviders(ui: React.ReactElement): RenderResult {
  return render(<ToastProvider>{ui}</ToastProvider>);
}
