import { mockIPC } from "@tauri-apps/api/mocks";
import fixture from "../../ipc/fixtures/ui-ready.json";
import { Panel } from "./panel";
export const used = [mockIPC, fixture, Panel];
