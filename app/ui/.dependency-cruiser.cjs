// UI import rules (design.md §2, IPC-2, R-1). Tested on a fixture tree in tools/arch.
const TESTS = "(\\.test\\.tsx?|^src/test-setup\\.ts)$";

/** @type {import('dependency-cruiser').IConfiguration} */
module.exports = {
  forbidden: [
    {
      name: "tauri-api-only-in-ipc",
      comment: "Only src/ipc/ calls invoke or creates a Channel (IPC-2).",
      severity: "error",
      from: { pathNot: "^src/ipc/" },
      to: { path: "@tauri-apps/api", pathNot: "@tauri-apps/api/mocks" },
    },
    {
      name: "ipc-only-from-actions",
      comment:
        "Only src/store/actions.ts calls the IPC module; generated types are free to import (R-1).",
      severity: "error",
      from: { pathNot: ["^src/ipc/", "^src/store/actions\\.ts$", TESTS] },
      to: { path: "^src/ipc/", pathNot: "^src/ipc/(bindings|fixtures)/" },
    },
    {
      name: "mocks-only-in-tests",
      comment: "mockIPC replaces the shell in tests only.",
      severity: "error",
      from: { pathNot: TESTS },
      to: { path: "@tauri-apps/api/mocks" },
    },
    {
      name: "fixtures-only-in-tests",
      comment: "Rust-written IPC fixtures feed the contract tests, never the app.",
      severity: "error",
      from: { pathNot: TESTS },
      to: { path: "^src/ipc/fixtures/" },
    },
    {
      name: "no-circular",
      severity: "error",
      from: {},
      to: { circular: true },
    },
  ],
  options: {
    doNotFollow: { path: "node_modules" },
    tsPreCompilationDeps: true,
    exclude: { path: "^src/ipc/bindings/" },
  },
};
