// The client side of the layout worker (E4 settings 5 and 6): elkjs runs in a module worker that
// starts with the app, and only ever lays out one view's unplaced elements or one behavior model.
// The request and result shapes are plain data, so the same `runLayout` serves the worker and the
// tests.
import { createContext, useContext } from "react";

export interface Point {
  x: number;
  y: number;
}

export interface LayoutPort {
  id: string;
  side: "left" | "right";
  // The port's center, from the top of its node.
  offset: number;
}

export interface LayoutNode {
  id: string;
  width: number;
  height: number;
  ports: LayoutPort[];
}

export interface LayoutEdge {
  id: string;
  // A node ID, or a port ID of one of the nodes.
  source: string;
  target: string;
}

export interface LayoutRequest {
  nodes: LayoutNode[];
  edges: LayoutEdge[];
  // Space between ELK's layers: wide on the canvas for relationship lines, narrow in a net.
  layerGap: number;
}

export interface LayoutResult {
  // Top-left corners, relative to the laid-out graph.
  positions: Record<string, Point>;
}

export interface LayoutClient {
  layout: (request: LayoutRequest) => Promise<LayoutResult>;
}

interface Reply {
  id: number;
  result?: LayoutResult;
  error?: string;
}

// Started once in main.tsx: spawning the worker and loading elkjs (1.5 MB) takes about 160 ms,
// so it happens before the first project opens (E4).
export function startLayoutWorker(): LayoutClient {
  const worker = new Worker(new URL("./layout.worker.ts", import.meta.url), { type: "module" });
  const pending = new Map<number, (reply: Reply) => void>();
  let next = 1;
  worker.addEventListener("message", (event: MessageEvent<unknown>) => {
    const reply = toReply(event.data);
    if (reply === null) {
      return;
    }
    pending.get(reply.id)?.(reply);
    pending.delete(reply.id);
  });
  return {
    layout: (request) => {
      const id = next;
      next += 1;
      return new Promise((resolve, reject) => {
        pending.set(id, (reply) => {
          if (reply.result === undefined) {
            reject(new Error(reply.error ?? "the layout worker sent no result"));
          } else {
            resolve(reply.result);
          }
        });
        worker.postMessage({ id, request });
      });
    },
  };
}

// The worker is our own code, but its messages are still `unknown` until narrowed (TS-1).
function toReply(data: unknown): Reply | null {
  if (typeof data !== "object" || data === null || !("id" in data)) {
    return null;
  }
  if (typeof data.id !== "number") {
    return null;
  }
  const reply: Reply = { id: data.id };
  if ("error" in data && typeof data.error === "string") {
    reply.error = data.error;
  }
  if ("result" in data) {
    const result = toResult(data.result);
    if (result !== null) {
      reply.result = result;
    }
  }
  return reply;
}

function toResult(data: unknown): LayoutResult | null {
  if (typeof data !== "object" || data === null || !("positions" in data)) {
    return null;
  }
  const { positions } = data;
  if (typeof positions !== "object" || positions === null) {
    return null;
  }
  const result: LayoutResult = { positions: {} };
  for (const [id, point] of Object.entries(positions)) {
    if (typeof point !== "object" || point === null || !("x" in point) || !("y" in point)) {
      return null;
    }
    if (typeof point.x !== "number" || typeof point.y !== "number") {
      return null;
    }
    result.positions[id] = { x: point.x, y: point.y };
  }
  return result;
}

// `null` until main.tsx provides the worker; without it nothing unplaced is drawn.
export const LayoutContext = createContext<LayoutClient | null>(null);

export function useLayoutClient(): LayoutClient | null {
  return useContext(LayoutContext);
}
