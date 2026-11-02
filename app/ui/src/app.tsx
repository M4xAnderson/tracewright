import { useEffect } from "react";
import { StartScreen } from "./features/start/start-screen";
import type { Screen } from "./ipc/bindings/screen";
import { reportScreenShown } from "./store/actions";
import { useAppStore } from "./store/app-store";

export function App() {
  const screen = useAppStore((s) => s.screen);

  // Synchronizes with the shell, which logs the screen and ends the smoke test on it. A one-shot
  // notification has nothing to undo, so there is no cleanup. A failed report is not swallowed:
  // it surfaces as an unhandled rejection, and the smoke test fails on the missing "UI ready".
  useEffect(() => {
    void reportScreenShown(screen);
  }, [screen]);

  return renderScreen(screen);
}

function renderScreen(screen: Screen) {
  switch (screen) {
    case "start":
      return <StartScreen />;
    default:
      return assertNever(screen);
  }
}

function assertNever(value: never): never {
  throw new Error(`unexpected screen: ${String(value)}`);
}
