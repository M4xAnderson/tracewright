// The only caller of the IPC module (R-1, IPC-2): components call actions, never `ipc/`.
import * as commands from "../ipc/commands";
import type { Screen } from "../ipc/bindings/screen";

export function reportScreenShown(screen: Screen): Promise<void> {
  return commands.uiReady({ screen });
}
