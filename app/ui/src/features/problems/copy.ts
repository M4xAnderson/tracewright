// Problems list and model-incomplete copy from the flows §5 catalog (PR-*, U-INC-*,
// V-BANNER-FNL), word for word. A kind without catalog copy shows the engine's cause.
import type { ProblemDto } from "../../ipc/bindings/problem-dto";
import type { View } from "../../ipc/bindings/view";
import { assertNever } from "../../store/app-store";
import { elementName, MODEL_INCOMPLETE, relationshipLabel } from "../verify/copy";

// `element/name` as "Billing: reserve". Element IDs never contain "/".
function qualifiedLabel(view: View, qualified: string | null): string {
  if (qualified === null) {
    return "(unknown)";
  }
  const slash = qualified.indexOf("/");
  if (slash < 0) {
    return qualified;
  }
  return `${elementName(view, qualified.slice(0, slash))}: ${qualified.slice(slash + 1)}`;
}

function bareName(qualified: string | null): string {
  if (qualified === null) {
    return "(unknown)";
  }
  return qualified.slice(qualified.indexOf("/") + 1);
}

function elementOf(view: View, problem: ProblemDto): string {
  return problem.element === null ? "(unknown)" : elementName(view, problem.element);
}

function fileOf(problem: ProblemDto): string {
  return problem.file ?? "a project file";
}

// U-INC-DANGLING: "Order API → (missing element) (calls)"
function danglingLabel(view: View, problem: ProblemDto): string {
  const relationship = view.relationships.find((r) => r.id === problem.relationship);
  const meaning = relationship?.meaning ?? null;
  const source = elementOf(view, problem);
  return `${source} → (missing element)${meaning === null ? "" : ` (${meaning})`}`;
}

// PR-*: one Problems list line.
export function problemText(view: View, problem: ProblemDto): string {
  switch (problem.kind) {
    case "parse":
    case "schema":
    case "tooLarge":
    case "behaviorNotLoaded":
      return problem.line === null
        ? `${fileOf(problem)}: ${problem.message}`
        : `${fileOf(problem)}, line ${problem.line}: ${problem.message}`;
    case "noBehavior":
      return `${elementOf(view, problem)} has no behavior model. Add one in ${fileOf(problem)}, then choose Reload.`;
    case "unbound":
      return `${qualifiedLabel(view, problem.port)} is not bound by any relationship. Bind it in the relationship inspector.`;
    case "unknownPort":
      return `${relationshipLabel(view, problem.relationship ?? "")} uses the port ${qualifiedLabel(view, problem.port)}, which does not exist. Choose an existing port in the relationship inspector.`;
    case "noWhenFull":
      return `${elementOf(view, problem)}: place ${bareName(problem.place)} has no when full choice. Add when_full (block, drop or overflow) in ${fileOf(problem)}, then choose Reload.`;
    case "nameClash":
      return `${elementOf(view, problem)} has both a place and a transition named ${bareName(problem.place)}.`;
    case "finalNotChecked":
      return `${fileOf(problem)} has a [final] table. Done when is checked from slice 1; until then, checks run without it.`;
    case "dangling":
      if (problem.relationship !== null) {
        return `${danglingLabel(view, problem)} has no target. Connect it to an element or delete it.`;
      }
      return problem.message;
    case "notAPort":
    case "bindingSelf":
    case "bindingAsync":
    case "noCapacity":
    case "badInitial":
    case "bindingDirection":
    case "bindingDuplicate":
    case "badBinding":
    case "duplicateId":
    case "parentCycle":
    case "property":
      return problem.message;
    default:
      return assertNever(problem.kind);
  }
}

// U-INC-*: one cause of a scope that is model incomplete.
function causeText(view: View, cause: ProblemDto): string {
  switch (cause.kind) {
    case "parse":
    case "schema":
    case "tooLarge":
      return `${fileOf(cause)} did not load.`;
    case "behaviorNotLoaded":
      return `${elementOf(view, cause)} has a behavior model that did not load.`;
    default:
      return problemText(view, cause);
  }
}

// U-INC followed by one line per cause.
export function incompleteLines(view: View, causes: ProblemDto[]): string[] {
  return [MODEL_INCOMPLETE, ...causes.map((cause) => causeText(view, cause))];
}

// The files the Session could not load (file-format §10.5).
export function filesNotLoaded(view: View): ProblemDto[] {
  return view.problems.filter(
    (problem) =>
      problem.kind === "parse" || problem.kind === "schema" || problem.kind === "tooLarge",
  );
}

// V-BANNER-FNL, with one "has an error" clause per file.
export function bannerText(files: ProblemDto[]): string {
  const clauses = files.map((file) =>
    file.line === null
      ? `${fileOf(file)} has an error (${file.message}).`
      : `${fileOf(file)} has an error (line ${file.line}: ${file.message}).`,
  );
  const fix = files.length === 1 ? "Fix it and choose Reload." : "Fix them and choose Reload.";
  return ["Verdicts are for the last version that loaded.", ...clauses, fix].join(" ");
}

export const NO_PROBLEMS = "No problems. Every file loaded and every port is bound.";
