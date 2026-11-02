import type { CSSProperties } from "react";
import { Button } from "../../components/ui/button";
import { Kbd } from "../../components/ui/kbd";
import { Progress } from "../../components/ui/progress";
import { openProjectFromDialog } from "../../store/actions";
import { useAppStore } from "../../store/app-store";
import { commandErrorText } from "./command-error-text";

// The folder name of a path, for "Opening <name>…".
function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter((part) => part !== "");
  return parts.at(-1) ?? path;
}

// Start screen (flows §4.1): the app name and Open project. New project comes with its ticket.
export function StartScreen() {
  const open = useAppStore((s) => s.open);
  if (open.kind === "opening") {
    // Shown only if opening takes longer than 500 ms (flows §3.3).
    return (
      <main
        className="appear-after flex h-full flex-col items-center justify-center gap-3"
        style={{ "--appear-after": "500ms" } as CSSProperties}
      >
        <p className="text-base font-semibold">{folderName(open.path)}</p>
        <p className="text-text-secondary">Opening {folderName(open.path)}…</p>
        <Progress value={null} aria-label={`Opening ${folderName(open.path)}`} className="w-64" />
      </main>
    );
  }
  return (
    <main className="flex h-full flex-col items-center justify-center gap-6">
      <h1 className="text-2xl font-semibold tracking-tight">Tracewright</h1>
      {open.kind === "failed" && (
        <p role="alert" className="max-w-[75ch] text-center text-problem">
          {commandErrorText(open.error)}
        </p>
      )}
      <Button variant="primary" size="large" onClick={() => void openProjectFromDialog()}>
        Open project
        <Kbd aria-hidden="true">Ctrl+O</Kbd>
      </Button>
    </main>
  );
}
