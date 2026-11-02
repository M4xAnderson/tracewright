// The only caller of the IPC module (R-1, IPC-2): components call actions, never `ipc/`.
import * as commands from "../ipc/commands";
import type { EngineEvent } from "../ipc/bindings/engine-event";
import type { Screen } from "../ipc/bindings/screen";
import { reduceEngineEvent, useAppStore } from "./app-store";

export function reportScreenShown(screen: Screen): Promise<void> {
  return commands.uiReady({ screen });
}

function applyEngineEvent(event: EngineEvent) {
  useAppStore.setState((state) => reduceEngineEvent(state, event));
}

// Open project (flows §4.1): the folder dialog first, so cancelling costs nothing; then the
// project opens and every property is queued for a check (ADR 0005).
export async function openProjectFromDialog(): Promise<void> {
  if (useAppStore.getState().open.kind === "opening") {
    return;
  }
  let path: string | null;
  try {
    path = await commands.pickProjectFolder();
  } catch (rejection) {
    useAppStore.setState({ open: { kind: "failed", error: commands.toCommandError(rejection) } });
    return;
  }
  if (path === null) {
    return;
  }
  useAppStore.setState({ open: { kind: "opening", path } });
  try {
    const opened = await commands.openProject(path, applyEngineEvent);
    useAppStore.setState({
      screen: "project",
      open: { kind: "idle" },
      project: { name: opened.name, path: opened.path },
      view: opened.view,
      currentKeys: null,
      verdicts: {},
      running: [],
      finishedInRun: [],
      message: null,
    });
  } catch (rejection) {
    useAppStore.setState({ open: { kind: "failed", error: commands.toCommandError(rejection) } });
    return;
  }
  await checkAll();
}

// Check (flows §4.7): every property of the view's scope. Keys already running are not
// restarted, so pressing Check again while checks run does nothing.
export async function checkAll(): Promise<void> {
  if (useAppStore.getState().running.length > 0) {
    return;
  }
  try {
    await commands.runChecks();
  } catch (rejection) {
    useAppStore.setState({ message: { kind: "error", error: commands.toCommandError(rejection) } });
  }
}
