import { useAppStore } from "../../store/app-store";
import type { AppState, Message } from "../../store/app-store";
import { assertNever } from "../../store/app-store";
import { commandErrorText } from "../start/command-error-text";
import { propertyName } from "../verify/copy";
import { verdictView } from "../verify/verdict-view";

const WORD = { proved: "Proved", disproved: "Disproved", unknown: "Unknown" } as const;

// "Checks finished: Deadlock-free Disproved, No capacity exceeded Proved." Only current verdicts
// are announced (flows §2.6, §3.1).
function messageText(state: AppState, message: Message): string {
  switch (message.kind) {
    case "checksFinished": {
      const parts = message.properties.flatMap((id) => {
        const property = state.view?.properties.find((p) => p.id === id);
        const row = verdictView(state, id);
        if (property === undefined || row.kind !== "current") {
          return [];
        }
        return [`${propertyName(property.kind)} ${WORD[row.verdict.kind]}`];
      });
      return parts.length === 0 ? "" : `Checks finished: ${parts.join(", ")}.`;
    }
    case "error":
      return commandErrorText(message.error);
    default:
      return assertNever(message);
  }
}

// Status bar (flows §2.6): the latest message, announced politely; background activity, never
// announced.
export function StatusBar() {
  const text = useAppStore((s) => (s.message === null ? "" : messageText(s, s.message)));
  const running = useAppStore((s) => s.running.length);
  return (
    <footer
      aria-label="Messages"
      className="flex h-6 shrink-0 items-center gap-3 border-t border-border-hairline bg-surface-panel px-3.5 text-[12px] text-text-secondary"
    >
      <output className="min-w-0 flex-1 truncate">{text}</output>
      {running > 0 && (
        <span>Checking {running === 1 ? "1 property" : `${running} properties`}…</span>
      )}
    </footer>
  );
}
