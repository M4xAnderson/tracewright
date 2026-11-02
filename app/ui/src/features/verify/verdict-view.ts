// The one verdict selector (flows §3.1). Every surface that shows a verdict uses it.
import type { UnknownDto } from "../../ipc/bindings/unknown-dto";
import type { VerdictDto } from "../../ipc/bindings/verdict-dto";
import type { AppState } from "../../store/app-store";

export type RowState =
  | { kind: "pending" }
  | { kind: "noBehavior" }
  | { kind: "unknownKey"; reason: UnknownDto }
  | { kind: "running" }
  | { kind: "current"; verdict: VerdictDto }
  | { kind: "noVerdict"; behaviorChanged: boolean };

type Inputs = Pick<AppState, "view" | "currentKeys" | "verdicts" | "running">;

// keysFresh comes first, so no branch renders from keys of another revision (design §4).
export function verdictView(state: Inputs, property: string): RowState {
  const { view, currentKeys } = state;
  if (view === null || currentKeys === null || currentKeys.rev !== view.rev) {
    return { kind: "pending" };
  }
  const keyState = currentKeys.keys[property];
  if (keyState === undefined) {
    return { kind: "noVerdict", behaviorChanged: false };
  }
  if (keyState === "nothingToCheck") {
    return { kind: "noBehavior" };
  }
  if ("unknown" in keyState) {
    return { kind: "unknownKey", reason: keyState.unknown };
  }
  if (state.running.includes(keyState.key)) {
    return { kind: "running" };
  }
  const last = state.verdicts[property];
  if (last === undefined) {
    return { kind: "noVerdict", behaviorChanged: false };
  }
  if (last.key === keyState.key) {
    return { kind: "current", verdict: last.verdict };
  }
  // Slice 0 drops a verdict for another key; slice 1 shows it as stale.
  return { kind: "noVerdict", behaviorChanged: true };
}
