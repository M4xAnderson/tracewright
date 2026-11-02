import { useAppStore } from "../../store/app-store";
import { ProblemsPane } from "../problems/problems-pane";
import { StatusBar } from "../status/status-bar";
import { VerdictPanel } from "../verify/verdict-panel";
import { ProjectHeader } from "./project-header";

// The main window of an open project (flows §2.2). The canvas and the inspector come with their
// tickets; the canvas keeps its place, with the bottom pane's Problems tab below it.
export function ProjectScreen() {
  const view = useAppStore((s) => s.view);
  if (view === null) {
    return null;
  }
  return (
    <div className="flex h-full flex-col">
      <ProjectHeader />
      <div className="flex min-h-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col">
          <section aria-label="Canvas" className="min-h-0 min-w-0 flex-1 bg-surface-canvas" />
          <ProblemsPane view={view} />
        </div>
        <VerdictPanel view={view} />
      </div>
      <StatusBar />
    </div>
  );
}
