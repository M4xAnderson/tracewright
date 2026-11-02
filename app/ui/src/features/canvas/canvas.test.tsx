// Seam C: open a project, see its Container view on the canvas and the selected element's behavior
// model in the bottom pane. The shell is mockIPC fed with the Rust-written `Opened` fixtures; the
// layout runs elkjs in process instead of in its worker.
import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, test } from "vitest";
import { App } from "../../app";
import openedOppositeLocks from "../../ipc/fixtures/opened-opposite-locks.json";
import openedWorkerPool from "../../ipc/fixtures/opened-worker-pool.json";
import { useAppStore } from "../../store/app-store";
import { runLayout } from "./layout";
import { type LayoutClient, type LayoutRequest, LayoutContext } from "./layout-client";

function inProcessLayout(requests: LayoutRequest[] = []): LayoutClient {
  return {
    layout: (request) => {
      requests.push(request);
      return runLayout(request);
    },
  };
}

async function openProject(opened: unknown, layout: LayoutClient = inProcessLayout()) {
  mockIPC((cmd) => {
    switch (cmd) {
      case "pick_project_folder":
        return "/home/designer/project";
      case "open_project":
        return opened;
      default:
        return undefined;
    }
  });
  render(
    <LayoutContext value={layout}>
      <App />
    </LayoutContext>,
  );
  await userEvent.click(screen.getByRole("button", { name: "Open project" }));
  return screen.findByRole("region", { name: "Canvas" });
}

function bottomPane() {
  return screen.getByRole("region", { name: /^Bottom pane/ });
}

beforeEach(() => {
  useAppStore.setState(useAppStore.getInitialState(), true);
});

describe("the Container view on the canvas", () => {
  test("shows the system's children by name and kind, with their ports", async () => {
    const canvas = await openProject(openedOppositeLocks);

    const lock = await within(canvas).findByRole("button", { name: "Lock service, container" });
    expect(lock).toHaveAccessibleDescription("Ports: acq_x, acq_y, rel_xy.");
    expect(within(canvas).getByRole("button", { name: "Component A, container" })).toBeVisible();
    // Component B has no layout file, so it appears once the layout worker placed it.
    expect(
      await within(canvas).findByRole("button", { name: "Component B, container" }),
    ).toBeVisible();
  });

  test("lays out only the unplaced elements, once per view", async () => {
    const requests: LayoutRequest[] = [];
    const canvas = await openProject(openedOppositeLocks, inProcessLayout(requests));
    await within(canvas).findByRole("button", { name: "Component B, container" });

    expect(requests.map((r) => r.nodes.map((n) => n.id))).toEqual([["b"]]);
  });

  test("Tab moves through the elements in reading order, and Enter selects one", async () => {
    const canvas = await openProject(openedOppositeLocks);
    const b = await within(canvas).findByRole("button", { name: "Component B, container" });
    const a = within(canvas).getByRole("button", { name: "Component A, container" });
    const lock = within(canvas).getByRole("button", { name: "Lock service, container" });

    a.focus();
    await userEvent.tab();
    expect(lock).toHaveFocus();
    await userEvent.tab();
    expect(b).toHaveFocus();
    await userEvent.tab({ shift: true });
    expect(lock).toHaveFocus();

    await userEvent.keyboard("{Enter}");
    expect(lock).toHaveAttribute("aria-pressed", "true");
    expect(
      within(bottomPane()).getByRole("heading", { name: "Behavior model: Lock service" }),
    ).toBeVisible();

    await userEvent.keyboard("{Escape}");
    expect(lock).toHaveAttribute("aria-pressed", "false");
    expect(bottomPane()).toHaveTextContent("Select an element to see its behavior model.");
  });

  test("Go to relationship selects a relationship without a pointer", async () => {
    await openProject(openedWorkerPool);

    await userEvent.keyboard("{Control>}{Shift>}o{/Shift}{/Control}");
    const dialog = await screen.findByRole("dialog", { name: "Go to relationship" });
    await userEvent.type(within(dialog).getByRole("combobox"), "back");
    await userEvent.keyboard("{Enter}");

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(bottomPane()).toHaveTextContent(
      "Clients → Worker pool (gives the worker back to) joins Clients: finish and Worker pool: release into one step.",
    );
  });

  test("Go to element selects an element and focuses it", async () => {
    const canvas = await openProject(openedWorkerPool);

    await userEvent.keyboard("{Control>}p{/Control}");
    const dialog = await screen.findByRole("dialog", { name: "Go to element" });
    await userEvent.type(within(dialog).getByRole("combobox"), "work");
    await userEvent.keyboard("{Enter}");

    const pool = await within(canvas).findByRole("button", { name: "Worker pool, container" });
    expect(pool).toHaveAttribute("aria-pressed", "true");
    expect(pool).toHaveFocus();
  });
});

describe("the behavior model pane", () => {
  test("asks for a selection while nothing is selected", async () => {
    await openProject(openedOppositeLocks);

    expect(bottomPane()).toHaveAccessibleName("Bottom pane: Behavior model");
    expect(bottomPane()).toHaveTextContent("Select an element to see its behavior model.");
  });

  test("shows the selected element's Petri net read-only: tokens, capacity, when full, ports", async () => {
    const canvas = await openProject(openedWorkerPool);
    await userEvent.click(
      await within(canvas).findByRole("button", { name: "Clients, container" }),
    );

    const pane = bottomPane();
    expect(within(pane).getByRole("heading", { name: "Behavior model: Clients" })).toBeVisible();
    const net = await within(pane).findByRole("figure", { name: "Petri net of Clients" });
    expect(
      await within(net).findByRole("group", { name: "Place idle: 3 tokens, capacity 3 · block" }),
    ).toHaveTextContent("3idlecapacity 3 · block");
    expect(
      within(net).getByRole("group", { name: "Place running: 0 tokens, capacity 3 · block" }),
    ).toBeVisible();
    expect(within(net).getByRole("group", { name: "Transition start, port" })).toHaveTextContent(
      "startport",
    );
  });

  test("an element without a behavior model says where to add one", async () => {
    const opened = structuredClone(openedWorkerPool);
    const pool = opened.view.elements.find((e) => e.id === "w");
    if (pool === undefined) {
      throw new Error("the fixture has the worker pool");
    }
    Object.assign(pool, { behavior: null });
    const canvas = await openProject(opened);

    const element = await within(canvas).findByRole("button", { name: "Worker pool, container" });
    expect(element).toHaveTextContent("No behavior model");
    await userEvent.click(element);
    expect(bottomPane()).toHaveTextContent(
      "No behavior model. Add it in elements/w.toml, then choose Reload.",
    );
  });

  test("an element whose behavior model did not load says so, on the canvas and in the pane", async () => {
    const opened = structuredClone(openedWorkerPool);
    const pool = opened.view.elements.find((e) => e.id === "w");
    if (pool === undefined) {
      throw new Error("the fixture has the worker pool");
    }
    Object.assign(pool, {
      behavior: { kind: "unloaded", message: "capacity must be at least 1", line: 7 },
    });
    const canvas = await openProject(opened);

    const element = await within(canvas).findByRole("button", { name: "Worker pool, container" });
    expect(element).toHaveTextContent("File has an error");
    expect(element).toHaveAccessibleDescription("The behavior model did not load.");
    await userEvent.click(element);
    expect(bottomPane()).toHaveTextContent(
      "The behavior model in elements/w.toml did not load (line 7): capacity must be at least 1. Fix it, then choose Reload.",
    );
  });
});
