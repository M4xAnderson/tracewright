// Seam C: a folder that does not open (flows §4.1). The shell is replaced by mockIPC and rejects
// open_project with the errors the Rust tests wrote from real folders.
import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";
import { App } from "../../app";
import openFailed from "../../ipc/fixtures/open-failed.json";
import openNewerFormat from "../../ipc/fixtures/open-newer-format.json";
import openNotAProject from "../../ipc/fixtures/open-not-a-project.json";
import openPermissionDenied from "../../ipc/fixtures/open-permission-denied.json";
import { useAppStore } from "../../store/app-store";

async function openFolderThatFails(error: unknown) {
  useAppStore.setState(useAppStore.getInitialState(), true);
  mockIPC((cmd) => {
    if (cmd === "pick_project_folder") {
      return "/home/designer/notes";
    }
    if (cmd === "open_project") {
      throw error;
    }
    return undefined;
  });
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open project" }));
  return screen.findByRole("alert");
}

describe("a folder that does not open shows why on the start screen and does not open", () => {
  test.each([
    [
      "not a project",
      openNotAProject,
      "/home/designer/notes is not a project: arch.toml is missing. Choose a project folder or create a new project.",
    ],
    [
      "unreadable",
      openPermissionDenied,
      "Could not open /home/designer/notes: you do not have permission to read it. Choose another folder or change its permissions.",
    ],
    [
      "saved by a newer format",
      openNewerFormat,
      "/home/designer/notes was saved by a newer version of this app (format 7). Update the app to open it.",
    ],
    [
      "any other cause",
      openFailed,
      "Could not open /home/designer/notes: arch.toml: format is missing; add `format = 1`. Check that the folder is readable, then open it again.",
    ],
  ])("%s", async (_case, error, text) => {
    const alert = await openFolderThatFails(error);

    expect(alert).toHaveTextContent(text);
    expect(screen.getByRole("heading", { level: 1, name: "Tracewright" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open project" })).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Verdicts" })).not.toBeInTheDocument();
  });
});
