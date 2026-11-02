// The only module that calls `invoke` (IPC-2). Argument names match the Rust command parameters.
import { invoke } from "@tauri-apps/api/core";
import type { UiReady } from "./bindings/ui-ready";

export function uiReady(ready: UiReady): Promise<void> {
  return invoke("ui_ready", { ready });
}
