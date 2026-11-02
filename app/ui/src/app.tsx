import { useEffect } from "react";
import { ProjectScreen } from "./features/project/project-screen";
import { StartScreen } from "./features/start/start-screen";
import type { Screen } from "./ipc/bindings/screen";
import { checkAll, openProjectFromDialog, reportScreenShown } from "./store/actions";
import { assertNever, useAppStore } from "./store/app-store";

export function App() {
  const screen = useAppStore((s) => s.screen);

  // Synchronizes with the shell, which logs the screen and ends the smoke test on it. A one-shot
  // notification has nothing to undo, so there is no cleanup. A failed report is not swallowed:
  // it surfaces as an unhandled rejection, and the smoke test fails on the missing "UI ready".
  useEffect(() => {
    void reportScreenShown(screen);
  }, [screen]);

  // Global shortcuts (flows §6.2): Ctrl+O everywhere, Ctrl+Enter with a project open.
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (!event.ctrlKey || event.altKey || event.shiftKey || event.metaKey) {
        return;
      }
      if (event.key === "o") {
        event.preventDefault();
        void openProjectFromDialog();
      } else if (event.key === "Enter" && useAppStore.getState().screen === "project") {
        event.preventDefault();
        void checkAll();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  return renderScreen(screen);
}

function renderScreen(screen: Screen) {
  switch (screen) {
    case "start":
      return <StartScreen />;
    case "project":
      return <ProjectScreen />;
    default:
      return assertNever(screen);
  }
}
