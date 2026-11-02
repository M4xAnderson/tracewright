// The canvas projection (E4 setting 4): the view the Rust core sent, mapped to what React Flow
// draws, keeping every object whose element or relationship did not change.
import { describe, expect, test } from "vitest";
import type { View } from "../../ipc/bindings/view";
import openedOppositeLocks from "../../ipc/fixtures/opened-opposite-locks.json";
import openedWorkerPool from "../../ipc/fixtures/opened-worker-pool.json";
import { placementFromLayout, projectCanvas, unplacedLayoutRequest } from "./canvas-model";
import { runLayout } from "./layout";

// JSON imports widen string literals; the fixtures were serialized from typed Rust values.
const oppositeLocks = openedOppositeLocks.view as View;
const workerPool = openedWorkerPool.view as View;

function node(projection: ReturnType<typeof projectCanvas>, id: string) {
  const found = projection.nodes.find((n) => n.id === id);
  if (found === undefined) {
    throw new Error(`no node ${id}`);
  }
  return found;
}

describe("canvas projection", () => {
  test("an element is drawn at the position of its layout file", () => {
    const projection = projectCanvas(null, oppositeLocks, {});
    expect(node(projection, "a").position).toEqual({ x: 0, y: 0 });
    expect(node(projection, "l").position).toEqual({ x: 440, y: 120 });
  });

  test("an unplaced element waits for the layout and is then drawn where it was placed", () => {
    expect(projectCanvas(null, oppositeLocks, {}).nodes.map((n) => n.id)).not.toContain("b");
    const placed = projectCanvas(null, oppositeLocks, { b: { x: 0, y: 400 } });
    expect(node(placed, "b").position).toEqual({ x: 0, y: 400 });
  });

  test("a file position wins over a layout result, so nothing placed ever moves", () => {
    const projection = projectCanvas(null, oppositeLocks, { a: { x: 999, y: 999 } });
    expect(node(projection, "a").position).toEqual({ x: 0, y: 0 });
  });

  test("only unplaced elements go to the layout, with the relationships between them", () => {
    const request = unplacedLayoutRequest(oppositeLocks);
    expect(request?.nodes.map((n) => n.id)).toEqual(["b"]);
    expect(request?.edges).toEqual([]);
    const allPlaced = projectCanvas(null, oppositeLocks, { b: { x: 0, y: 400 } });
    expect(allPlaced.nodes).toHaveLength(3);
    const noneUnplaced: View = {
      ...oppositeLocks,
      elements: oppositeLocks.elements.filter((e) => e.id !== "b"),
    };
    expect(unplacedLayoutRequest(noneUnplaced)).toBeNull();
  });

  test("ports are handles: the target's on the left, the source's on the right", () => {
    const projection = projectCanvas(null, oppositeLocks, { b: { x: 0, y: 400 } });
    const sides = (id: string) =>
      node(projection, id).data.ports.map((p) => `${p.side} ${p.name} ${p.kind}`);
    expect(sides("a")).toEqual([
      "right release transition",
      "right take_x transition",
      "right take_y transition",
    ]);
    expect(sides("l")).toEqual([
      "left acq_x transition",
      "left acq_y transition",
      "left rel_xy transition",
    ]);
  });

  test("a bound relationship attaches to its port handles", () => {
    const projection = projectCanvas(null, oppositeLocks, { b: { x: 0, y: 400 } });
    const edge = projection.edges.find((e) => e.id === "ra1");
    expect(edge).toMatchObject({
      source: "a",
      target: "l",
      sourceHandle: "port:take_x",
      targetHandle: "port:acq_x",
    });
  });

  test("a relationship is labeled with its meaning and named from end to end", () => {
    const projection = projectCanvas(null, workerPool, { c: { x: 0, y: 0 }, w: { x: 400, y: 0 } });
    const edge = projection.edges.find((e) => e.id === "r1");
    expect(edge?.data?.label).toBe("takes a worker from");
    expect(edge?.ariaLabel).toBe("Clients → Worker pool (takes a worker from)");
  });

  test("elements are listed in reading order: rows top to bottom, left to right", () => {
    const projection = projectCanvas(null, oppositeLocks, { b: { x: 0, y: 400 } });
    expect(projection.nodes.map((n) => n.id)).toEqual(["a", "l", "b"]);
    const oneRow = projectCanvas(null, oppositeLocks, { b: { x: 300, y: 10 } });
    expect(oneRow.nodes.map((n) => n.id)).toEqual(["a", "b", "l"]);
  });

  test("a new view keeps the objects of unchanged elements and relationships", () => {
    const placed = { b: { x: 0, y: 400 } };
    const before = projectCanvas(null, oppositeLocks, placed);
    const renamed: View = {
      ...structuredClone(oppositeLocks),
      rev: 1,
      elements: oppositeLocks.elements.map((e) =>
        e.id === "b" ? { ...e, name: "Worker B" } : structuredClone(e),
      ),
    };
    const after = projectCanvas(before, renamed, placed);
    expect(node(after, "a")).toBe(node(before, "a"));
    expect(node(after, "l")).toBe(node(before, "l"));
    expect(node(after, "b")).not.toBe(node(before, "b"));
    expect(node(after, "b").data.name).toBe("Worker B");
    const edge = (p: typeof before, id: string) => p.edges.find((e) => e.id === id);
    expect(edge(after, "ra1")).toBe(edge(before, "ra1"));
    expect(edge(after, "rb1")).not.toBe(edge(before, "rb1"));
    expect(edge(after, "rb1")?.ariaLabel).toBe("Worker B → Lock service");
    expect(projectCanvas(after, structuredClone(renamed), placed)).toBe(after);
  });

  test("the layout places unplaced elements below the placed ones, the same way every time", async () => {
    const request = unplacedLayoutRequest(oppositeLocks);
    if (request === null) {
      throw new Error("b is unplaced");
    }
    const result = await runLayout(request);
    expect(await runLayout(request)).toEqual(result);
    const placed = placementFromLayout(oppositeLocks, result);
    // l is the lowest placed element: y 120 plus its height of 128 (three ports).
    expect(placed["b"]?.y).toBeGreaterThanOrEqual(120 + 128);
    expect(placed["b"]?.x).toBe(0);
  });

  test("an unplaced view is laid out from the origin without overlaps", async () => {
    const unplaced: View = {
      ...workerPool,
      elements: workerPool.elements.map((e) => ({ ...e, position: null })),
    };
    const request = unplacedLayoutRequest(unplaced);
    if (request === null) {
      throw new Error("every element is unplaced");
    }
    const placed = placementFromLayout(unplaced, await runLayout(request));
    const projection = projectCanvas(null, unplaced, placed);
    const [c, w] = [node(projection, "c"), node(projection, "w")];
    expect(Math.min(c.position.x, w.position.x)).toBeGreaterThanOrEqual(0);
    const apart =
      c.position.x + (c.width ?? 0) <= w.position.x ||
      w.position.x + (w.width ?? 0) <= c.position.x;
    expect(apart).toBe(true);
  });
});
