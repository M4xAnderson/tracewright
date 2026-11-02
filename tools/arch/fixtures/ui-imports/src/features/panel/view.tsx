import { mockIPC } from "@tauri-apps/api/mocks";
import fixture from "../../ipc/fixtures/ui-ready.json";
export const View = () => [mockIPC, fixture];
