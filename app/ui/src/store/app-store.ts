import { create } from "zustand";
import type { CheckKey } from "../ipc/bindings/check-key";
import type { CommandError } from "../ipc/bindings/command-error";
import type { CurrentKeys } from "../ipc/bindings/current-keys";
import type { EngineEvent } from "../ipc/bindings/engine-event";
import type { Screen } from "../ipc/bindings/screen";
import type { VerdictDto } from "../ipc/bindings/verdict-dto";
import type { View } from "../ipc/bindings/view";

export type OpenState =
  | { kind: "idle" }
  | { kind: "opening"; path: string }
  | { kind: "failed"; error: CommandError };

// The latest status bar message (flows §2.6). It keeps IDs; the status bar renders the words.
export type Message =
  | { kind: "checksFinished"; properties: string[] }
  | { kind: "error"; error: CommandError };

export interface AppState {
  screen: Screen;
  open: OpenState;
  project: { name: string; path: string } | null;
  // Read projection (R-1): replaced from core responses only. Rev 0 until edits exist.
  view: View | null;
  currentKeys: CurrentKeys | null;
  // The last verdict per property ID, with the key it was computed for (design §5).
  verdicts: Partial<Record<string, { key: CheckKey; verdict: VerdictDto }>>;
  // Keys of checks queued or running.
  running: CheckKey[];
  // Properties whose verdict arrived since the running set was last empty.
  finishedInRun: string[];
  message: Message | null;
}

const initialState: AppState = {
  screen: "start",
  open: { kind: "idle" },
  project: null,
  view: null,
  currentKeys: null,
  verdicts: {},
  running: [],
  finishedInRun: [],
  message: null,
};

// The model itself never lives here; it stays in the Rust core.
export const useAppStore = create<AppState>()(() => initialState);

// One engine event applied to the store (design §5): currentKeys is latest-wins by rev, verdicts
// are kept by property with their key, and whether one is current is decided in render.
export function reduceEngineEvent(state: AppState, event: EngineEvent): Partial<AppState> {
  switch (event.kind) {
    case "currentKeys": {
      if (state.currentKeys !== null && event.rev < state.currentKeys.rev) {
        return {};
      }
      return { currentKeys: { rev: event.rev, keys: event.keys } };
    }
    case "checkQueued": {
      if (state.running.includes(event.key)) {
        return {};
      }
      return {
        running: [...state.running, event.key],
        finishedInRun: state.running.length === 0 ? [] : state.finishedInRun,
      };
    }
    case "verdict": {
      const running = state.running.filter((key) => key !== event.key);
      const finishedInRun = [...state.finishedInRun, event.property];
      return {
        verdicts: {
          ...state.verdicts,
          [event.property]: { key: event.key, verdict: event.verdict },
        },
        running,
        finishedInRun,
        ...(running.length === 0
          ? { message: { kind: "checksFinished", properties: finishedInRun } }
          : {}),
      };
    }
    default:
      return assertNever(event);
  }
}

export function assertNever(value: never): never {
  throw new Error(`unexpected value: ${JSON.stringify(value)}`);
}
