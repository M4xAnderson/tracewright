// The crate graph of ADR 0004 as amended by ADR 0009, checked on `cargo metadata`.
// cargo-deny's `wrappers` see only direct parents, so this walks the resolved graph instead.

/** The part of `cargo metadata --format-version 1` this check reads. */
export interface CargoMetadata {
  workspace_members: string[];
  packages: { id: string; name: string }[];
  resolve: {
    nodes: {
      id: string;
      deps: { pkg: string; dep_kinds: { kind: "dev" | "build" | null }[] }[];
    }[];
  };
}

const SHELL = "tracewright-app";

// Every workspace crate and the workspace crates it may depend on, in any dependency kind.
// A dev-dependency counts too: recheck's tests must not borrow verify's semantics either.
const ALLOWED_WORKSPACE_DEPS: Record<string, readonly string[]> = {
  "tracewright-model": [],
  "tracewright-net": [],
  "tracewright-verify": ["tracewright-net"],
  "tracewright-recheck": ["tracewright-net"],
  "tracewright-format": ["tracewright-model"],
  "tracewright-scan": ["tracewright-model"],
  "tracewright-engine": [
    "tracewright-model",
    "tracewright-net",
    "tracewright-verify",
    "tracewright-recheck",
    "tracewright-format",
    "tracewright-scan",
  ],
  // interop is development-only: no crate lists it, so nothing in the app's graph reaches it.
  "tracewright-interop": ["tracewright-net"],
  [SHELL]: ["tracewright-engine", "tracewright-model"],
};

// Workspace crates a crate may use only in its tests. The TAPAAL corpus in interop's tests
// compares both explorers with verifypn (NS §18 item 10); interop's own code stays on `net`.
const ALLOWED_DEV_DEPS: Record<string, readonly string[]> = {
  "tracewright-interop": ["tracewright-verify", "tracewright-recheck"],
};

const RUNTIMES = ["tokio", "tauri"];

/** Returns one message per violated rule; an empty list means the graph is clean. */
export function checkCrateEdges(meta: CargoMetadata): string[] {
  const nameOf = new Map(meta.packages.map((p) => [p.id, p.name]));
  const nodeOf = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const name = (id: string) => nameOf.get(id) ?? id;
  const members = new Set(meta.workspace_members);
  const problems: string[] = [];

  for (const id of meta.workspace_members) {
    const crate = name(id);
    const allowed = ALLOWED_WORKSPACE_DEPS[crate];
    if (allowed === undefined) {
      problems.push(`${crate} is not a crate of ADR 0004 or 0009; add it to the rules with an ADR`);
      continue;
    }
    const allowedInTests = ALLOWED_DEV_DEPS[crate] ?? [];
    for (const dep of nodeOf.get(id)?.deps ?? []) {
      const onlyInTests = dep.dep_kinds.every((k) => k.kind === "dev");
      const ok =
        allowed.includes(name(dep.pkg)) || (onlyInTests && allowedInTests.includes(name(dep.pkg)));
      if (members.has(dep.pkg) && !ok) {
        problems.push(`${crate} must not depend on ${name(dep.pkg)} (ADR 0004, 0009)`);
      }
    }
    if (crate !== SHELL) {
      for (const runtime of reachedRuntimes(id, nodeOf, name)) {
        problems.push(`${crate} reaches ${runtime}, which is allowed only in ${SHELL} (ADR 0004)`);
      }
    }
  }
  return problems;
}

// Walks normal and build dependencies (what is compiled into the crate) breadth-first.
function reachedRuntimes(
  start: string,
  nodeOf: Map<string, CargoMetadata["resolve"]["nodes"][number]>,
  name: (id: string) => string,
): string[] {
  const seen = new Set([start]);
  const queue = [start];
  const found = new Set<string>();
  while (queue.length > 0) {
    const id = queue.shift() as string;
    for (const dep of nodeOf.get(id)?.deps ?? []) {
      const compiledIn = dep.dep_kinds.some((k) => k.kind !== "dev");
      if (!compiledIn || seen.has(dep.pkg)) {
        continue;
      }
      seen.add(dep.pkg);
      queue.push(dep.pkg);
      if (RUNTIMES.includes(name(dep.pkg))) {
        found.add(name(dep.pkg));
      }
    }
  }
  return RUNTIMES.filter((r) => found.has(r));
}
