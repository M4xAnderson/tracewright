import { useAppStore } from "../../store/app-store";
import { BehaviorPane } from "../behavior/behavior-pane";
import { Canvas } from "../canvas/canvas";
import { StatusBar } from "../status/status-bar";
import { VerdictPanel } from "../verify/verdict-panel";
import { ProjectHeader } from "./project-header";

// The main window of an open project (flows §2.2): the canvas over the bottom pane, the verdict
// panel on the right. The inspector and the other bottom pane tabs come with their tickets.
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
          <section aria-label="Canvas" className="min-h-60 flex-1 bg-surface-canvas">
            <Canvas view={view} />
          </section>
          <BehaviorPane view={view} />
        </div>
        <VerdictPanel view={view} />
      </div>
      <StatusBar />
    </div>
  );
}
