// View actions (R-1): shared view state that never reaches the core.
import { type BottomTab, type Selection, useAppStore } from "./app-store";

export function select(selection: Selection | null): void {
  useAppStore.setState({ selection });
}

export function showBottomTab(bottomTab: BottomTab): void {
  useAppStore.setState({ bottomTab });
}
