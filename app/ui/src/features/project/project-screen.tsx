import { useAppStore } from "../../store/app-store";
import { StatusBar } from "../status/status-bar";
import { VerdictPanel } from "../verify/verdict-panel";
import { ProjectHeader } from "./project-header";

// The main window of an open project (flows §2.2). The canvas, the bottom pane and the inspector
// come with their tickets; the canvas keeps its place.
export function ProjectScreen() {
  const view = useAppStore((s) => s.view);
  if (view === null) {
    return null;
  }
  return (
    <div className="flex h-full flex-col">
      <ProjectHeader />
      <div className="flex min-h-0 flex-1">
        <section aria-label="Canvas" className="min-w-0 flex-1 bg-surface-canvas" />
        <VerdictPanel view={view} />
      </div>
      <StatusBar />
    </div>
  );
}
