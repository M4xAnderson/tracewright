// Seam C: a project that loads with problems opens and says so (flows §4.1, P-OPEN-PROBLEMS).
import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";
import { App } from "../../app";
import openedBrokenFile from "../../ipc/fixtures/opened-broken-file.json";
import openedHalfDone from "../../ipc/fixtures/opened-half-done.json";
import openedOneJob from "../../ipc/fixtures/opened-one-job.json";
import { useAppStore } from "../../store/app-store";

async function open(opened: unknown) {
  useAppStore.setState(useAppStore.getInitialState(), true);
  mockIPC((cmd) => {
    if (cmd === "pick_project_folder") {
      return "/home/designer/project";
    }
    return cmd === "open_project" ? opened : undefined;
  });
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open project" }));
  await screen.findByRole("region", { name: "Verdicts" });
  return screen.getByRole("contentinfo", { name: "Messages" }).textContent;
}

describe("opening a project that loads with problems", () => {
  test.each([
    [openedHalfDone, "Opened half-done with 7 problems. See the Problems list."],
    [openedBrokenFile, "Opened broken-file with 1 problem. See the Problems list."],
  ])("names the number of problems and opens the Problems tab", async (opened, text) => {
    expect(await open(opened)).toBe(text);
    expect(screen.getByRole("region", { name: "Bottom pane: Problems" })).toBeInTheDocument();
  });

  test("a project without problems opens without a message", async () => {
    expect(await open(openedOneJob)).toBe("");
  });
});
