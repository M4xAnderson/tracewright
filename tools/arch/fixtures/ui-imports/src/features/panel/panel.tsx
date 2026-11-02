import { invoke } from "@tauri-apps/api/core";
import { shown } from "../../ipc/commands";
import type { Screen } from "../../ipc/bindings/screen";
import { report } from "../../store/actions";
export const Panel = (screen: Screen) => [invoke, shown, report, screen];
