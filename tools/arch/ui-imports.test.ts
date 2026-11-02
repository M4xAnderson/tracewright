import { expect, test } from "bun:test";
import { join } from "node:path";

const root = join(import.meta.dir, "..", "..");
const ui = join(root, "app", "ui");

interface Violation {
  rule: { name: string };
  from: string;
  to: string;
}

// Runs the UI's real dependency-cruiser config on a fixture tree with allowed and forbidden imports.
function violations(): string[] {
  const run = Bun.spawnSync(
    [
      join(ui, "node_modules", ".bin", "depcruise"),
      "src",
      "--config",
      join(ui, ".dependency-cruiser.cjs"),
      "--output-type",
      "json",
    ],
    { cwd: join(import.meta.dir, "fixtures", "ui-imports") },
  );
  const stdout = run.stdout.toString();
  if (stdout === "") {
    throw new Error(`depcruise printed nothing: ${run.stderr.toString()}`);
  }
  const result = JSON.parse(stdout) as { summary: { violations: Violation[] } };
  return result.summary.violations
    .map((v) => `${v.rule.name}: ${v.from} -> ${v.to.replace(/^.*node_modules\//, "")}`)
    .sort();
}

test("only ipc/ imports the Tauri API, only actions import ipc/, only tests use mocks and fixtures", () => {
  expect(violations()).toEqual([
    "fixtures-only-in-tests: src/features/panel/view.tsx -> src/ipc/fixtures/ui-ready.json",
    "ipc-only-from-actions: src/features/panel/panel.tsx -> src/ipc/commands.ts",
    "ipc-only-from-actions: src/store/app-store.ts -> src/ipc/commands.ts",
    "mocks-only-in-tests: src/features/panel/view.tsx -> @tauri-apps/api/mocks",
    "tauri-api-only-in-ipc: src/features/panel/panel.tsx -> @tauri-apps/api/core",
  ]);
});
