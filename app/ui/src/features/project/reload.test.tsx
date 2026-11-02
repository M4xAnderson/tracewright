// Seam C: Reload (flows §4.5). The shell is replaced by mockIPC and answers with the replies and
// events the Rust tests wrote while reloading a real folder edited on disk.
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";
import { App } from "../../app";
import type { EngineEvent } from "../../ipc/bindings/engine-event";
import eventsOneJob from "../../ipc/fixtures/events-one-job.json";
import openedOneJob from "../../ipc/fixtures/opened-one-job.json";
import reloadEventsOneJob from "../../ipc/fixtures/reload-events-one-job.json";
import reloadEventsParseError from "../../ipc/fixtures/reload-events-parse-error.json";
import reloadFailed from "../../ipc/fixtures/reload-failed.json";
import reloadNothingChanged from "../../ipc/fixtures/reload-nothing-changed.json";
import reloadOneJob from "../../ipc/fixtures/reload-one-job.json";
import reloadParseError from "../../ipc/fixtures/reload-parse-error.json";
import { useAppStore } from "../../store/app-store";

// JSON imports widen string literals; the fixtures were serialized from typed Rust values.
const events = (json: unknown) => json as EngineEvent[];

// one-job opens and its check runs to a verdict; reload_project then answers with `reply` (or
// rejects with `error`) after emitting `reloadEvents`, as the Session does.
async function openOneJobThenReload(options: {
  reply?: unknown;
  error?: unknown;
  reloadEvents?: EngineEvent[];
}) {
  useAppStore.setState(useAppStore.getInitialState(), true);
  let onmessage: ((event: EngineEvent) => void) | null = null;
  const emit = (emitted: EngineEvent[]) => {
    for (const event of emitted) {
      act(() => onmessage?.(event));
    }
  };
  const calls: string[] = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    switch (cmd) {
      case "pick_project_folder":
        return "/home/designer/one-job";
      case "open_project": {
        const channel = (args as { events?: unknown }).events;
        if (typeof channel === "object" && channel !== null && "onmessage" in channel) {
          onmessage = channel.onmessage as (event: EngineEvent) => void;
        }
        return openedOneJob;
      }
      case "reload_project":
        emit(options.reloadEvents ?? []);
        if (options.error !== undefined) {
          throw options.error;
        }
        return options.reply;
      default:
        return undefined;
    }
  });
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open project" }));
  await screen.findByRole("region", { name: "Verdicts" });
  emit(events(eventsOneJob));
  return calls;
}

function statusText() {
  return screen.getByRole("contentinfo", { name: "Messages" }).textContent;
}

function verdictRow(property: string) {
  return within(screen.getByRole("region", { name: "Verdicts" })).getByRole("article", {
    name: property,
  });
}

describe("Reload re-reads the files edited on disk", () => {
  test("the header's Reload names the changed element and its verdict is no longer current", async () => {
    await openOneJobThenReload({
      reply: reloadOneJob,
      reloadEvents: events(reloadEventsOneJob),
    });
    expect(verdictRow("Deadlock-free")).toHaveTextContent("Disproved");

    await userEvent.click(screen.getByRole("button", { name: "Reload" }));

    expect(statusText()).toBe("Reloaded. Changed: Nightly job.");
    expect(verdictRow("Deadlock-free")).toHaveTextContent("Behavior changed since the last check.");
    expect(verdictRow("Deadlock-free")).not.toHaveTextContent("Disproved");
  });

  test("Ctrl+R reloads instead of reloading the window", async () => {
    const calls = await openOneJobThenReload({ reply: reloadNothingChanged });

    const keydown = new KeyboardEvent("keydown", { key: "r", ctrlKey: true, cancelable: true });
    act(() => {
      window.dispatchEvent(keydown);
    });
    await screen.findByText("No files changed on disk.");

    expect(keydown.defaultPrevented).toBe(true);
    expect(calls).toContain("reload_project");
  });

  test("with nothing changed on disk it says so and keeps the verdicts", async () => {
    await openOneJobThenReload({ reply: reloadNothingChanged });

    await userEvent.click(screen.getByRole("button", { name: "Reload" }));

    expect(statusText()).toBe("No files changed on disk.");
    expect(verdictRow("Deadlock-free")).toHaveTextContent("Disproved");
  });

  test("a changed file that does not parse is named with its line and listed in Problems", async () => {
    await openOneJobThenReload({
      reply: reloadParseError,
      reloadEvents: events(reloadEventsParseError),
    });

    await userEvent.click(screen.getByRole("button", { name: "Reload" }));

    expect(statusText()).toBe(
      "elements/job.toml was not loaded: line 11: string values must be quoted, expected literal string. The app keeps the last version that loaded.",
    );
    const problems = screen.getByRole("region", { name: "Bottom pane: Problems" });
    expect(within(problems).getByRole("heading", { name: "Problems (1)" })).toBeInTheDocument();
    expect(problems).toHaveTextContent("elements/job.toml");
    expect(verdictRow("Deadlock-free")).toHaveTextContent("Unknown");
  });

  test("a folder that no longer opens leaves the project as it was", async () => {
    await openOneJobThenReload({ error: reloadFailed });

    await userEvent.click(screen.getByRole("button", { name: "Reload" }));

    expect(statusText()).toBe(
      "Could not reload: arch.toml: format version 2 is newer than this app supports (1). The app keeps the last version that loaded.",
    );
    expect(verdictRow("Deadlock-free")).toHaveTextContent("Disproved");
  });
});
