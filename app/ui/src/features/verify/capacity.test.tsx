// Seam C for no capacity exceeded and asynchronous bindings: open a project and read the verdict
// panel, with the shell replaced by mockIPC and fed with JSON the Rust tests wrote from real check
// runs (the IPC contract test). Rows that need a place inside a behavior model change one field of
// a real event, so the rest of the payload stays what the engine sends.
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";
import { App } from "../../app";
import type { EngineEvent } from "../../ipc/bindings/engine-event";
import type { PlaceOrigin } from "../../ipc/bindings/place-origin";
import type { View } from "../../ipc/bindings/view";
import type { WaitingDto } from "../../ipc/bindings/waiting-dto";
import eventsProducerConsumer from "../../ipc/fixtures/events-producer-consumer.json";
import eventsWorkerPool from "../../ipc/fixtures/events-worker-pool.json";
import openedProducerConsumer from "../../ipc/fixtures/opened-producer-consumer.json";
import openedWorkerPool from "../../ipc/fixtures/opened-worker-pool.json";
import { useAppStore } from "../../store/app-store";
import { waitingLine } from "./copy";

// JSON imports widen string literals; the fixtures were serialized from typed Rust values.
const events = (json: unknown) => json as EngineEvent[];

// Opens the project through the start screen and returns a way to emit engine events on the
// Channel the UI registered with open_project.
async function open(opened: unknown): Promise<(...emitted: EngineEvent[]) => void> {
  useAppStore.setState(useAppStore.getInitialState(), true);
  let onmessage: ((event: EngineEvent) => void) | null = null;
  mockIPC((cmd, args) => {
    if (cmd === "pick_project_folder") {
      return "/home/designer/project";
    }
    if (cmd === "open_project") {
      const channel = (args as { events?: unknown }).events;
      if (typeof channel === "object" && channel !== null && "onmessage" in channel) {
        onmessage = channel.onmessage as (event: EngineEvent) => void;
      }
      return opened;
    }
    return undefined;
  });
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open project" }));
  await screen.findByRole("region", { name: "Verdicts" });
  return (...emitted) => {
    for (const event of emitted) {
      act(() => {
        if (onmessage === null) {
          throw new Error("open_project registered no Channel");
        }
        onmessage(event);
      });
    }
  };
}

function row(property: string) {
  return within(screen.getByRole("region", { name: "Verdicts" })).getByRole("article", {
    name: property,
  });
}

const QUEUE: PlaceOrigin = { kind: "own", element: "billing", place: "queue" };

// The producer-consumer run with the place of both verdicts moved into Billing's own net.
function withOwnPlace(): EngineEvent[] {
  return events(eventsProducerConsumer).map((event) => {
    if (event.kind !== "verdict") {
      return event;
    }
    const { verdict } = event;
    if (verdict.kind === "disproved" && verdict.violation.kind === "overflow") {
      return {
        ...event,
        verdict: { ...verdict, violation: { ...verdict.violation, place: QUEUE } },
      };
    }
    if (verdict.kind === "unknown" && verdict.reason.category === "capacityExceededFirst") {
      return { ...event, verdict: { ...verdict, reason: { ...verdict.reason, place: QUEUE } } };
    }
    return event;
  });
}

describe("no capacity exceeded", () => {
  test("a queue without back-pressure is Disproved with the overflowing relationship and the steps", async () => {
    const emit = await open(openedProducerConsumer);

    emit(...events(eventsProducerConsumer));

    const disproved = row("No capacity exceeded");
    expect(within(disproved).getByText("Disproved")).toBeInTheDocument();
    expect(
      within(disproved).getByText(
        "Order API → Billing (publishes to) holds 4 messages; capacity 3.",
      ),
    ).toBeInTheDocument();
    const steps = within(disproved).getByRole("list", { name: "Counterexample steps" });
    expect(
      within(steps)
        .getAllByRole("listitem")
        .map((item) => item.textContent),
    ).toEqual([
      "Step 1 of 4: Order API: publish",
      "Step 2 of 4: Order API: publish",
      "Step 3 of 4: Order API: publish",
      "Step 4 of 4: Order API: publish",
    ]);
    expect(disproved).not.toHaveTextContent("deadlock");
  });

  test("a place inside a behavior model is named by its element and counted in tokens", async () => {
    const emit = await open(openedProducerConsumer);

    emit(...withOwnPlace());

    expect(
      within(row("No capacity exceeded")).getByText("Billing: queue holds 4 tokens; capacity 3."),
    ).toBeInTheDocument();
  });

  test("a scope without an overflow place is Proved by the certificate with its limits", async () => {
    const emit = await open(openedWorkerPool);

    emit(...events(eventsWorkerPool));

    const proved = row("No capacity exceeded");
    expect(within(proved).getByText("Proved")).toBeInTheDocument();
    const lines = [
      "Worker pool never goes over any capacity.",
      "Places that block or drop can still make senders wait or lose messages.",
      "Proved with a certificate; a second explorer re-checked the certificate.",
      "Holds for every order of steps with the capacities and initial tokens in this model. Time and rates are not modeled.",
    ];
    for (const line of lines) {
      expect(within(proved).getByText(line)).toBeInTheDocument();
    }
    expect(proved).not.toHaveTextContent("reachable markings");
    expect(proved).not.toHaveTextContent("never deadlocks");
  });
});

describe("deadlock-free with an overflow place", () => {
  test("it stops at the first overflow: Unknown capacity exceeded first, with the next step", async () => {
    const emit = await open(openedProducerConsumer);

    emit(...events(eventsProducerConsumer));

    const unknown = row("Deadlock-free");
    expect(within(unknown).getByText("Unknown")).toBeInTheDocument();
    expect(unknown).toHaveTextContent(
      "Deadlock-free was decided only up to the first time Order API → Billing (publishes to) exceeds capacity 3. Set when full to block or drop for this relationship, or raise its capacity.",
    );
    expect(unknown).not.toHaveTextContent("Proved");
    expect(screen.getByRole("status")).toHaveTextContent(
      "Checks finished: No capacity exceeded Disproved, Deadlock-free Unknown.",
    );
  });

  test("for a place inside a behavior model the next step names its file", async () => {
    const emit = await open(openedProducerConsumer);

    emit(...withOwnPlace());

    expect(row("Deadlock-free")).toHaveTextContent(
      "Deadlock-free was decided only up to the first time Billing: queue exceeds capacity 3. Set when full to block or drop for queue in elements/billing.toml, or raise its capacity, then choose Reload.",
    );
  });
});

describe("waiting lines at a full generated place", () => {
  test("a sender waits for room in the relationship, counted in messages", () => {
    const view = openedProducerConsumer.view as View;
    const waiting: WaitingDto = {
      transition: { kind: "own", element: "api", transition: "publish" },
      waits: [
        {
          place: { kind: "generated", relationship: "orders" },
          reason: { kind: "outputFull", capacity: 3, holds: 3 },
        },
      ],
    };

    expect(waitingLine(view, waiting)).toBe(
      "Order API waits at publish for room in Order API → Billing (publishes to): 3 of 3 messages, when full: block.",
    );
  });
});
