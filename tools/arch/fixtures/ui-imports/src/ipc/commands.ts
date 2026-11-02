import { invoke } from "@tauri-apps/api/core";
import type { Screen } from "./bindings/screen";
export const shown = (screen: Screen) => invoke("ui_ready", { ready: { screen } });
