import { LoaderCircle, Play } from "lucide-react";
import { Button } from "../../components/ui/button";
import { Kbd } from "../../components/ui/kbd";
import { checkAll } from "../../store/actions";
import { useAppStore } from "../../store/app-store";
import { elementName } from "../verify/copy";

// Header (flows §2.5): project name, view title and level, Check. Save, Reload, the summary
// chips and Commands come with their tickets.
export function ProjectHeader() {
  const project = useAppStore((s) => s.project);
  const view = useAppStore((s) => s.view);
  const checking = useAppStore((s) => s.running.length > 0);
  if (project === null || view === null) {
    return null;
  }
  const title = view.system === null ? null : elementName(view, view.system.id);
  return (
    <header
      aria-label="Header"
      className="flex h-11 shrink-0 items-center gap-3 border-b border-border-hairline bg-surface-panel pr-3 pl-3.5"
    >
      <span className="font-semibold">{project.name}</span>
      <span aria-hidden="true" className="h-[18px] w-px bg-border-hairline" />
      {title !== null && <span className="font-medium">{title}</span>}
      <span className="text-text-secondary">
        {title !== null && <span aria-hidden="true">· </span>}Level: Container
      </span>
      <span className="flex-1" />
      <Button variant="primary" onClick={() => void checkAll()}>
        {checking ? (
          <>
            <LoaderCircle aria-hidden="true" className="animate-spin" />
            Checking…
          </>
        ) : (
          <>
            <Play aria-hidden="true" />
            Check
            <Kbd aria-hidden="true">Ctrl+Enter</Kbd>
          </>
        )}
      </Button>
    </header>
  );
}
