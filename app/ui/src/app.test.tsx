import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, test } from "vitest";
import { App } from "./app";
// Written by the Rust test `ipc::tests::ui_ready_fixture` (the IPC contract, seam C).
import uiReadyFixture from "./ipc/fixtures/ui-ready.json";

describe("App", () => {
  test("shows the start screen with the app name", () => {
    mockIPC(() => undefined);

    render(<App />);

    expect(screen.getByRole("heading", { level: 1, name: "Tracewright" })).toBeInTheDocument();
  });

  test("tells the shell that the start screen is shown, in the shape Rust expects", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
    });

    render(<App />);

    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0]).toEqual({ cmd: "ui_ready", args: { ready: uiReadyFixture } });
  });
});
