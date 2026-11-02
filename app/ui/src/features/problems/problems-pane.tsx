// The Problems tab of the bottom pane (flows §2.1, §4.5): what stops files from loading or checks
// from running, one line per cause, sorted by file, then line (the engine sends them sorted).
// Problems are never verdicts and never findings (flows §2.1).
import { CircleAlert, TriangleAlert } from "lucide-react";
import type { ProblemDto } from "../../ipc/bindings/problem-dto";
import type { View } from "../../ipc/bindings/view";
import { assertNever } from "../../store/app-store";
import { elementName } from "../verify/copy";
import { NO_PROBLEMS, problemText } from "./copy";

export function ProblemsPane({ view }: { view: View }) {
  const problems = view.problems;
  return (
    <section
      aria-label="Bottom pane: Problems"
      className="flex h-[280px] max-h-[40vh] shrink-0 flex-col border-t border-border-hairline bg-surface-panel"
    >
      <h2 className="px-4 py-2 text-[11px] font-semibold tracking-wide text-text-muted uppercase">
        Problems ({problems.length})
      </h2>
      {problems.length === 0 ? (
        <p className="px-4 text-text-secondary">{NO_PROBLEMS}</p>
      ) : (
        <ul className="min-h-0 flex-1 overflow-y-auto">
          {problems.map((problem) => (
            <ProblemLine
              key={`${problem.file}|${problem.kind}|${problem.message}`}
              view={view}
              problem={problem}
            />
          ))}
        </ul>
      )}
    </section>
  );
}

function ProblemLine({ view, problem }: { view: View; problem: ProblemDto }) {
  const where = [problem.file, problem.element === null ? null : elementName(view, problem.element)]
    .filter((part) => part !== null)
    .join(" · ");
  return (
    <li className="grid grid-cols-[22px_1fr] gap-x-2 border-t border-border-hairline px-4 py-2">
      <Severity severity={problem.severity} />
      <div className="max-w-[75ch]">
        <p>{problemText(view, problem)}</p>
        <p className="text-[12px] text-text-muted">{where}</p>
      </div>
    </li>
  );
}

// Icon plus text, never color alone (UI-1).
function Severity({ severity }: { severity: ProblemDto["severity"] }) {
  switch (severity) {
    case "error":
      return (
        <span className="pt-0.5">
          <CircleAlert aria-hidden="true" className="size-4 text-verdict-disproved" />
          <span className="sr-only">Error: </span>
        </span>
      );
    case "warning":
      return (
        <span className="pt-0.5">
          <TriangleAlert aria-hidden="true" className="size-4 text-text-muted" />
          <span className="sr-only">Warning: </span>
        </span>
      );
    default:
      return assertNever(severity);
  }
}
