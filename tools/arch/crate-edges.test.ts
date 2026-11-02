import { describe, expect, test } from "bun:test";
import { checkCrateEdges, type CargoMetadata } from "./crate-edges.ts";

type Kind = "normal" | "dev" | "build";
type Edge = [from: string, to: string, kind?: Kind];

const WORKSPACE = [
  "tracewright-model",
  "tracewright-net",
  "tracewright-verify",
  "tracewright-recheck",
  "tracewright-format",
  "tracewright-scan",
  "tracewright-engine",
  "tracewright-interop",
  "tracewright-app",
];

// The edges ADR 0004 and 0009 allow, plus the shell's third-party runtime.
const ALLOWED: Edge[] = [
  ["tracewright-verify", "tracewright-net"],
  ["tracewright-recheck", "tracewright-net"],
  ["tracewright-format", "tracewright-model"],
  ["tracewright-scan", "tracewright-model"],
  ["tracewright-engine", "tracewright-model"],
  ["tracewright-engine", "tracewright-net"],
  ["tracewright-engine", "tracewright-verify"],
  ["tracewright-engine", "tracewright-recheck"],
  ["tracewright-engine", "tracewright-format"],
  ["tracewright-engine", "tracewright-scan"],
  ["tracewright-interop", "tracewright-net"],
  ["tracewright-interop", "tracewright-verify", "dev"],
  ["tracewright-interop", "tracewright-recheck", "dev"],
  ["tracewright-app", "tracewright-engine"],
  ["tracewright-app", "tauri"],
  ["tauri", "tokio"],
];

function metadata(edges: Edge[]): CargoMetadata {
  const id = (name: string) => `${name} 0.0.0 (id)`;
  const names = new Set([...WORKSPACE, ...edges.flatMap(([from, to]) => [from, to])]);
  return {
    workspace_members: WORKSPACE.map(id),
    packages: [...names].map((name) => ({ id: id(name), name })),
    resolve: {
      nodes: [...names].map((name) => ({
        id: id(name),
        deps: edges
          .filter(([from]) => from === name)
          .map(([, to, kind = "normal"]) => ({
            pkg: id(to),
            dep_kinds: [{ kind: kind === "normal" ? null : kind }],
          })),
      })),
    },
  };
}

describe("checkCrateEdges", () => {
  test("accepts the crate graph of ADR 0004 and 0009", () => {
    expect(checkCrateEdges(metadata(ALLOWED))).toEqual([]);
  });

  test.each([
    ["tracewright-recheck", "tracewright-verify"],
    ["tracewright-recheck", "tracewright-engine"],
    ["tracewright-verify", "tracewright-recheck"],
    ["tracewright-verify", "tracewright-model"],
  ])("rejects %s depending on %s", (from, to) => {
    expect(checkCrateEdges(metadata([...ALLOWED, [from, to]]))).toEqual([
      `${from} must not depend on ${to} (ADR 0004, 0009)`,
    ]);
  });

  test.each([
    ["tracewright-engine", "normal"],
    ["tracewright-app", "normal"],
    ["tracewright-app", "dev"],
    ["tracewright-verify", "dev"],
  ] as const)("keeps the development-only interop out of %s (%s)", (from, kind) => {
    const edges: Edge[] = [...ALLOWED, [from, "tracewright-interop", kind]];
    expect(checkCrateEdges(metadata(edges))).toEqual([
      `${from} must not depend on tracewright-interop (ADR 0004, 0009)`,
    ]);
  });

  test.each(["tracewright-verify", "tracewright-recheck"])(
    "allows %s only as a dev-dependency of interop, for the TAPAAL corpus",
    (to) => {
      const edges: Edge[] = [...ALLOWED, ["tracewright-interop", to]];
      expect(checkCrateEdges(metadata(edges))).toEqual([
        `tracewright-interop must not depend on ${to} (ADR 0004, 0009)`,
      ]);
    },
  );

  test("rejects a forbidden edge that is only a dev-dependency", () => {
    const edges: Edge[] = [...ALLOWED, ["tracewright-recheck", "tracewright-verify", "dev"]];
    expect(checkCrateEdges(metadata(edges))).toEqual([
      "tracewright-recheck must not depend on tracewright-verify (ADR 0004, 0009)",
    ]);
  });

  test.each(["tokio", "tauri"])("rejects %s in a core crate", (runtime) => {
    const edges: Edge[] = [...ALLOWED, ["tracewright-engine", runtime]];
    expect(checkCrateEdges(metadata(edges))).toContain(
      `tracewright-engine reaches ${runtime}, which is allowed only in tracewright-app (ADR 0004)`,
    );
  });

  test("rejects tokio reached through a third-party crate, in every crate above it", () => {
    const edges: Edge[] = [...ALLOWED, ["tracewright-format", "some-io"], ["some-io", "tokio"]];
    expect(checkCrateEdges(metadata(edges))).toEqual([
      "tracewright-format reaches tokio, which is allowed only in tracewright-app (ADR 0004)",
      "tracewright-engine reaches tokio, which is allowed only in tracewright-app (ADR 0004)",
    ]);
  });

  test("ignores tokio as a dev-dependency of a core crate's tests", () => {
    const edges: Edge[] = [...ALLOWED, ["tracewright-engine", "tokio", "dev"]];
    expect(checkCrateEdges(metadata(edges))).toEqual([]);
  });

  test("rejects a workspace crate the rules do not know", () => {
    const meta = metadata(ALLOWED);
    meta.workspace_members.push("tracewright-cli 0.0.0 (id)");
    meta.packages.push({ id: "tracewright-cli 0.0.0 (id)", name: "tracewright-cli" });
    meta.resolve.nodes.push({ id: "tracewright-cli 0.0.0 (id)", deps: [] });
    expect(checkCrateEdges(meta)).toEqual([
      "tracewright-cli is not a crate of ADR 0004 or 0009; add it to the rules with an ADR",
    ]);
  });
});
