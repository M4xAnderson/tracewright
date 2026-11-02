// Seam C: open a project from the start screen and read the verdict panel, with the shell replaced
// by mockIPC and fed with JSON the Rust tests wrote from real check runs (the IPC contract test).
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";
import { App } from "../../app";
import type { EngineEvent } from "../../ipc/bindings/engine-event";
import eventsOneJob from "../../ipc/fixtures/events-one-job.json";
import eventsOppositeLocks from "../../ipc/fixtures/events-opposite-locks.json";
import eventsWorkerPool from "../../ipc/fixtures/events-worker-pool.json";
import openFailed from "../../ipc/fixtures/open-failed.json";
import openedOneJob from "../../ipc/fixtures/opened-one-job.json";
import openedOppositeLocks from "../../ipc/fixtures/opened-opposite-locks.json";
import openedWorkerPool from "../../ipc/fixtures/opened-worker-pool.json";
import { useAppStore } from "../../store/app-store";

// JSON imports widen string literals; the fixtures were serialized from typed Rust values.
const events = (json: unknown) => json as EngineEvent[];

// The first events of a check run of no-deadlock: the key set, its checkQueued and its verdict.
function deadlockFreeRun(json: unknown) {
  const all = events(json);
  const currentKeys = all.find((e) => e.kind === "currentKeys");
  const queued = all.find((e) => e.kind === "checkQueued" && e.property === "no-deadlock");
  const verdict = all.find((e) => e.kind === "verdict" && e.property === "no-deadlock");
  if (currentKeys?.kind !== "currentKeys" || queued === undefined || verdict?.kind !== "verdict") {
    throw new Error("the fixture holds currentKeys, checkQueued and a verdict for no-deadlock");
  }
  return { currentKeys, queued, verdict };
}

interface Shell {
  emit: (...events: EngineEvent[]) => void;
  calls: string[];
}

// The Tauri shell: the folder dialog returns `picked`, open_project answers with `opened` or
// rejects with `openError`, and the test emits engine events on the Channel the UI registered.
function fakeShell(options: {
  picked: string | null;
  opened?: unknown;
  openError?: unknown;
}): Shell {
  let onmessage: ((event: EngineEvent) => void) | null = null;
  const calls: string[] = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    switch (cmd) {
      case "pick_project_folder":
        return options.picked;
      case "open_project": {
        const channel = (args as { events?: unknown }).events;
        if (typeof channel === "object" && channel !== null && "onmessage" in channel) {
          onmessage = channel.onmessage as (event: EngineEvent) => void;
        }
        if (options.openError !== undefined) {
          throw options.openError;
        }
        return options.opened;
      }
      default:
        return undefined;
    }
  });
  return {
    calls,
    emit: (...emitted) => {
      for (const event of emitted) {
        act(() => {
          if (onmessage === null) {
            throw new Error("open_project registered no Channel");
          }
          onmessage(event);
        });
      }
    },
  };
}

async function openProject(shell: Shell) {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open project" }));
  await screen.findByRole("region", { name: "Verdicts" });
  return shell;
}

function row(property: string) {
  return within(screen.getByRole("region", { name: "Verdicts" })).getByRole("article", {
    name: property,
  });
}

function resetStore() {
  useAppStore.setState(useAppStore.getInitialState(), true);
}

describe("open a project and see the verdict", () => {
  test("the start screen offers Open project; a cancelled dialog changes nothing", async () => {
    resetStore();
    const shell = fakeShell({ picked: null });
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "Open project" }));

    expect(screen.getByRole("heading", { level: 1, name: "Tracewright" })).toBeInTheDocument();
    expect(shell.calls).not.toContain("open_project");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  test("a folder that does not open shows the error inline on the start screen", async () => {
    resetStore();
    fakeShell({ picked: "/home/designer/notes", openError: openFailed });
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "Open project" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not open /home/designer/notes: this folder is not a project: it has no arch.toml. Check that the folder is readable, then open it again.",
    );
    expect(screen.getByRole("button", { name: "Open project" })).toBeInTheDocument();
  });

  test("opening shows the project, its view and level, and runs the checks", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/e1b-opposite-locks", opened: openedOppositeLocks }),
    );

    const header = screen.getByRole("banner", { name: "Header" });
    expect(header).toHaveTextContent("e1b-opposite-locks");
    expect(header).toHaveTextContent("Two components, two locks");
    expect(header).toHaveTextContent("Level: Container");
    expect(shell.calls.filter((cmd) => cmd !== "ui_ready")).toEqual([
      "pick_project_folder",
      "open_project",
      "run_checks",
    ]);
    expect(row("Deadlock-free")).toHaveTextContent("Two components, two locks · Level: Container");
  });

  test("the opposite locks are Disproved with the deadlock sentence, the waiting lines and the steps", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/e1b-opposite-locks", opened: openedOppositeLocks }),
    );

    shell.emit(...events(eventsOppositeLocks));

    const disproved = row("Deadlock-free");
    expect(within(disproved).getByText("Disproved")).toBeInTheDocument();
    expect(disproved).toHaveTextContent(
      "Two components, two locks can deadlock: after 2 steps, no step can happen.",
    );
    const waiting = within(disproved).getByRole("list", { name: "Waiting steps" });
    expect(within(waiting).getAllByRole("listitem")).toHaveLength(6);
    expect(within(waiting).getAllByRole("listitem")[0]).toHaveTextContent(
      "Component A → Lock service: take_x + acq_x waits for room in Component A: has_1: 1 of 1 tokens, when full: block, for a token in Component A: idle and for a token in Lock service: x.",
    );
    const steps = within(disproved).getByRole("list", { name: "Counterexample steps" });
    expect(
      within(steps)
        .getAllByRole("listitem")
        .map((item) => item.textContent),
    ).toEqual([
      "Step 1 of 2: Component A → Lock service: take_x + acq_x",
      "Step 2 of 2: Component B → Lock service: take_y + acq_y",
    ]);
    expect(disproved).not.toHaveTextContent("no final predicate");
    expect(screen.getByRole("status")).toHaveTextContent(
      "Checks finished: Deadlock-free Disproved, No capacity exceeded Proved.",
    );
  });

  test("a run that finishes adds the done-when caveat of slice 0", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/one-job", opened: openedOneJob }),
    );

    shell.emit(...events(eventsOneJob));

    const disproved = row("Deadlock-free");
    expect(disproved).toHaveTextContent("One job can deadlock: after 1 step, no step can happen.");
    expect(disproved).toHaveTextContent(
      "Job waits at work for room in done: 1 of 1 tokens, when full: block and for a token in todo.",
    );
    expect(disproved).toHaveTextContent(
      "No transition is enabled and no final predicate (done when) is defined, so this run stops here. If this is where the run is meant to end, done when will let you say so; it comes with slice 1. Until then, a run that finishes counts as a deadlock.",
    );
  });

  test("the worker pool is Proved with its headline, its limits, the markings and the scope line", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/e1c-worker-pool", opened: openedWorkerPool }),
    );

    shell.emit(...events(eventsWorkerPool));

    const proved = row("Deadlock-free");
    expect(within(proved).getByText("Proved")).toBeInTheDocument();
    const lines = [
      "Worker pool never deadlocks.",
      "No deadlock does not mean done: a timeout or retry loop can run forever.",
      "Checked all 3 reachable markings; re-checked by a second explorer.",
      "Holds for every order of steps with the capacities and initial tokens in this model. Time and rates are not modeled.",
    ];
    for (const line of lines) {
      expect(within(proved).getByText(line)).toBeInTheDocument();
    }
    expect(proved).not.toHaveTextContent("Assumes:");
  });

  test("a property this slice cannot check is Unknown with its reason, without a check", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/e1c-worker-pool", opened: openedWorkerPool }),
    );
    const { currentKeys } = deadlockFreeRun(eventsWorkerPool);

    // The key state the Session sends for a user property in slice 0 (NS V57).
    shell.emit({
      ...currentKeys,
      keys: {
        ...currentKeys.keys,
        "no-overflow": {
          unknown: {
            category: "propertyNotCheckable",
            details: ["user properties are checked from slice 4"],
          },
        },
      },
    });

    const unknown = row("No capacity exceeded");
    expect(within(unknown).getByText("Unknown")).toBeInTheDocument();
    expect(unknown).toHaveTextContent("Property cannot be checked:");
    expect(unknown).toHaveTextContent("user properties are checked from slice 4");
    expect(unknown).not.toHaveTextContent("Proved");
  });

  test("while a check runs, its row and the Check button say Checking…", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/e1b-opposite-locks", opened: openedOppositeLocks }),
    );
    const { currentKeys, queued } = deadlockFreeRun(eventsOppositeLocks);

    shell.emit(currentKeys, queued);

    expect(within(row("Deadlock-free")).getByText("Checking…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Checking…" })).toBeInTheDocument();
    expect(screen.getByText("Checking 1 property…")).toBeInTheDocument();
  });

  test("a running check counts the markings of each phase as it explores", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/e1b-opposite-locks", opened: openedOppositeLocks }),
    );
    const { currentKeys, queued } = deadlockFreeRun(eventsOppositeLocks);
    const progress = events(eventsOppositeLocks).find(
      (e) => e.kind === "checkProgress" && e.property === "no-deadlock",
    );
    if (queued.kind !== "checkQueued" || progress === undefined) {
      throw new Error("the fixture holds checkQueued and checkProgress for no-deadlock");
    }

    shell.emit(currentKeys, queued, progress);
    expect(row("Deadlock-free")).toHaveTextContent("Exploring… 1 marking");

    const { property, key } = queued;
    shell.emit({ kind: "checkProgress", property, key, phase: "main", markings: 120400 });
    expect(row("Deadlock-free")).toHaveTextContent("Exploring… 120,400 markings");

    shell.emit({ kind: "checkProgress", property, key, phase: "recheck", markings: 48100 });
    expect(row("Deadlock-free")).toHaveTextContent("Re-checking… 48,100 markings");
    expect(row("Deadlock-free")).not.toHaveTextContent("Exploring");
  });
});

describe("keysFresh: a verdict is current only once the keys of its revision arrived", () => {
  test("a verdict that arrives before currentKeys renders as pending, then as current", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/e1b-opposite-locks", opened: openedOppositeLocks }),
    );
    const { currentKeys, queued, verdict } = deadlockFreeRun(eventsOppositeLocks);

    shell.emit(queued, verdict);

    expect(row("Deadlock-free")).not.toHaveTextContent("Disproved");
    expect(row("Deadlock-free")).toHaveAttribute("aria-busy", "true");

    shell.emit(currentKeys);

    expect(within(row("Deadlock-free")).getByText("Disproved")).toBeInTheDocument();
    expect(row("Deadlock-free")).toHaveAttribute("aria-busy", "false");
  });

  test("keys of another revision leave every row pending", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/e1b-opposite-locks", opened: openedOppositeLocks }),
    );
    const { currentKeys, queued, verdict } = deadlockFreeRun(eventsOppositeLocks);

    shell.emit({ ...currentKeys, rev: currentKeys.rev + 1 }, queued, verdict);

    for (const property of ["Deadlock-free", "No capacity exceeded"]) {
      expect(row(property)).not.toHaveTextContent("Disproved");
      expect(row(property)).not.toHaveTextContent("Unknown");
      expect(row(property)).toHaveAttribute("aria-busy", "true");
    }
  });

  test("a verdict for another key is not shown as current", async () => {
    resetStore();
    const shell = await openProject(
      fakeShell({ picked: "/home/designer/e1b-opposite-locks", opened: openedOppositeLocks }),
    );
    const { currentKeys, verdict } = deadlockFreeRun(eventsOppositeLocks);

    shell.emit(currentKeys, { ...verdict, key: "0".repeat(64) });

    expect(row("Deadlock-free")).not.toHaveTextContent("Disproved");
    expect(row("Deadlock-free")).toHaveTextContent("Behavior changed since the last check.");
  });
});
