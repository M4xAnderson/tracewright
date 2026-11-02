import { create } from "zustand";
import type { Screen } from "../ipc/bindings/screen";

interface AppState {
  screen: Screen;
}

// Shared view state (R-1). The model itself never lives here; it stays in the Rust core.
export const useAppStore = create<AppState>()(() => ({
  screen: "start",
}));
