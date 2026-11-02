// The only module that calls `invoke` or creates a Channel (IPC-2). Argument names match the Rust
// command parameters.
import { Channel, invoke } from "@tauri-apps/api/core";
import type { CommandError } from "./bindings/command-error";
import type { EngineEvent } from "./bindings/engine-event";
import type { Opened } from "./bindings/opened";
import type { ReloadReply } from "./bindings/reload-reply";
import type { UiReady } from "./bindings/ui-ready";

export function uiReady(ready: UiReady): Promise<void> {
  return invoke("ui_ready", { ready });
}

// `null` when the user cancels the folder dialog.
export function pickProjectFolder(): Promise<string | null> {
  return invoke("pick_project_folder");
}

// The Session's events for this project arrive on `onEvent` until another project opens.
export function openProject(path: string, onEvent: (event: EngineEvent) => void): Promise<Opened> {
  const events = new Channel<EngineEvent>(onEvent);
  return invoke("open_project", { path, events });
}

export function runChecks(): Promise<void> {
  return invoke("run_checks");
}

// The new keys arrive as an event on the open project's Channel.
export function reloadProject(): Promise<ReloadReply> {
  return invoke("reload_project");
}

const COMMAND_ERROR_KINDS: ReadonlySet<string> = new Set<CommandError["kind"]>([
  "notAProject",
  "permissionDenied",
  "newerFormat",
  "openFailed",
  "reloadFailed",
  "invalidArgument",
  "noProjectOpen",
  "internal",
]);

// A rejected command carries the serialized CommandError (a generated IPC type, so its kind
// decides its shape); anything else is a bug in the shell or the bridge and becomes `internal`.
export function toCommandError(rejection: unknown): CommandError {
  if (typeof rejection === "object" && rejection !== null && "kind" in rejection) {
    const kind = rejection.kind;
    if (typeof kind === "string" && COMMAND_ERROR_KINDS.has(kind)) {
      return rejection as CommandError;
    }
  }
  return { kind: "internal", message: String(rejection) };
}
