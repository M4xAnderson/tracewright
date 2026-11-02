// Verdict copy from the flows §5 catalog, word for word. Names are resolved from the view at
// render time (design §3), so a rename never shows an old label.
import type { PlaceRef } from "../../ipc/bindings/place-ref";
import type { PropertyKind } from "../../ipc/bindings/property-kind";
import type { PropertyView } from "../../ipc/bindings/property-view";
import type { TransitionOrigin } from "../../ipc/bindings/transition-origin";
import type { UnknownDto } from "../../ipc/bindings/unknown-dto";
import type { View } from "../../ipc/bindings/view";
import type { WaitDto } from "../../ipc/bindings/wait-dto";
import type { WaitingDto } from "../../ipc/bindings/waiting-dto";
import { assertNever } from "../../store/app-store";

const count = new Intl.NumberFormat("en-US");

// The catalog names of the templates (PL §8.2); an unknown template shows its ID.
function templateName(template: string | null): string {
  switch (template) {
    case "never-overflows":
      return "Never overflows";
    case "never-more-than":
      return "Never more than";
    case "never-fills-up":
      return "Never fills up";
    case "pool-share":
      return "Never holds the whole pool";
    case "get-back-to":
      return "Can always get back to";
    default:
      return template ?? "Template";
  }
}

export function propertyName(property: Pick<PropertyView, "kind" | "template">): string {
  const kind: PropertyKind = property.kind;
  switch (kind) {
    case "deadlockFree":
      return "Deadlock-free";
    case "noCapacityExceeded":
      return "No capacity exceeded";
    case "noDeadTransitions":
      return "No dead transitions";
    case "never":
      return "Never";
    case "alwaysReachable":
      return "Always reachable";
    case "template":
      return templateName(property.template);
    default:
      return assertNever(kind);
  }
}

export function elementName(view: View, id: string): string {
  if (view.system?.id === id) {
    return view.system.name;
  }
  return view.elements.find((element) => element.id === id)?.name ?? id;
}

// "Order platform · Level: Container" (VRD-4). An own scope says so; the catalog has no own-scope
// example in slice 0.
export function scopeLine(view: View, property: PropertyView): string {
  const name = elementName(view, property.scope.element);
  const own = property.scope.behavior === "own" ? " (own behavior)" : "";
  return `${name}${own} · Level: Container`;
}

// "Order API → Billing (publishes to)"
export function relationshipLabel(view: View, id: string): string {
  const relationship = view.relationships.find((r) => r.id === id);
  if (relationship === undefined) {
    return id;
  }
  const ends = `${elementName(view, relationship.source)} → ${elementName(view, relationship.target)}`;
  return relationship.meaning === null ? ends : `${ends} (${relationship.meaning})`;
}

// A fused transition shows one name when caller and callee share it ("reserve"), else both.
function syncName(origin: Extract<TransitionOrigin, { kind: "sync" }>): string {
  const { caller, callee } = origin;
  return caller.transition === callee.transition
    ? caller.transition
    : `${caller.transition} + ${callee.transition}`;
}

// R-STEP-IN "Billing: take_lock" / R-STEP-SYNC "Order API → Billing (calls): reserve"
export function transitionLabel(view: View, origin: TransitionOrigin): string {
  switch (origin.kind) {
    case "own":
      return `${elementName(view, origin.element)}: ${origin.transition}`;
    case "sync":
      return `${relationshipLabel(view, origin.relationship)}: ${syncName(origin)}`;
    default:
      return assertNever(origin);
  }
}

export function stepText(view: View, step: number, of: number, origin: TransitionOrigin): string {
  return `Step ${step} of ${of}: ${transitionLabel(view, origin)}`;
}

// V-PRO-T1
export function provedHeadline(kind: PropertyKind, scope: string): string | null {
  return kind === "deadlockFree" ? `${scope} never deadlocks.` : null;
}

// V-DNM-T1
export function doesNotMean(kind: PropertyKind): string | null {
  return kind === "deadlockFree"
    ? "No deadlock does not mean done: a timeout or retry loop can run forever."
    : null;
}

// V-PRO-1, exhaustive
export function exhaustiveLine(markings: number): string {
  return `Checked all ${count.format(markings)} reachable markings; re-checked by a second explorer.`;
}

// V-PRO-ASM
export function assumptionLine(view: View, relationship: string): string {
  return `Assumes: ${relationshipLabel(view, relationship)} binds no ports, so it adds no behavior.`;
}

export const SCOPE_LINE =
  "Holds for every order of steps with the capacities and initial tokens in this model. Time and rates are not modeled.";

// V-DIS-T1
export function deadlockHeadline(scope: string, steps: number): string {
  return `${scope} can deadlock: after ${count.format(steps)} ${steps === 1 ? "step" : "steps"}, no step can happen.`;
}

export const NO_FINAL_S0 =
  "No transition is enabled and no final predicate (done when) is defined, so this run stops here. If this is where the run is meant to end, done when will let you say so; it comes with slice 1. Until then, a run that finishes counts as a deadlock.";

// A place in the waiting element is bare ("lock_b"); any other is qualified ("Billing: lock_b").
function placeLabel(view: View, place: PlaceRef, owner: string | null): string {
  return place.element === owner
    ? place.place
    : `${elementName(view, place.element)}: ${place.place}`;
}

// One clause of V-DIS-DL-EMPTY / V-DIS-DL-FULL. With several clauses the "it holds" note moves
// into parentheses, so the clauses still read as one list.
function clause(view: View, wait: WaitDto, owner: string | null, alone: boolean): string {
  const place = placeLabel(view, wait.place, owner);
  const reason = wait.reason;
  switch (reason.kind) {
    case "needsTokens": {
      const what =
        reason.needed === 1 ? `a token in ${place}` : `${reason.needed} tokens in ${place}`;
      if (reason.holds === 0) {
        return `for ${what}`;
      }
      return alone
        ? `for ${what}; it holds ${reason.holds}`
        : `for ${what} (it holds ${reason.holds})`;
    }
    case "outputFull":
      return `for room in ${place}: ${reason.holds} of ${reason.capacity} tokens, when full: block`;
    default:
      return assertNever(reason);
  }
}

function joinClauses(clauses: string[]): string {
  if (clauses.length <= 1) {
    return clauses.join("");
  }
  return `${clauses.slice(0, -1).join(", ")} and ${clauses.at(-1)}`;
}

// V-DIS-DL-EMPTY / -FULL / -SYNC: "Billing waits at take_lock for a token in lock_b."
export function waitingLine(view: View, waiting: WaitingDto): string {
  const origin = waiting.transition;
  const owner = origin.kind === "own" ? origin.element : null;
  const alone = waiting.waits.length === 1;
  const clauses = joinClauses(waiting.waits.map((wait) => clause(view, wait, owner, alone)));
  switch (origin.kind) {
    case "own":
      return `${elementName(view, origin.element)} waits at ${origin.transition} ${clauses}.`;
    case "sync":
      return `${relationshipLabel(view, origin.relationship)}: ${syncName(origin)} waits ${clauses}.`;
    default:
      return assertNever(origin);
  }
}

// U-INC
export const MODEL_INCOMPLETE =
  "Model incomplete: this view cannot be checked until these are fixed:";

// Flows §4.9: the reason in the user's terms. Lines the engine sends (details) follow the first.
export function unknownLines(reason: UnknownDto): string[] {
  switch (reason.category) {
    case "modelIncomplete":
      return [MODEL_INCOMPLETE, ...reason.details];
    case "propertyNotCheckable":
      return ["Property cannot be checked:", ...reason.details];
    case "internalError":
      return [
        "Internal error: this is a bug in the tool, not in your design. Details were saved to the local log.",
      ];
    case "internalDisagreement":
      return [
        "Internal disagreement: the two explorers disagreed about this model, so the tool gives no verdict. This is a bug in the tool, not in your design. Copy details and add this model to the differential corpus.",
      ];
    case "cancelled":
      return ["You cancelled this check."];
    case "budgetExhausted":
      return [
        reason.phase === "main"
          ? `Budget exhausted: explored ${count.format(reason.markings)} markings without deciding.`
          : `Budget exhausted while re-checking: the second explorer stopped after ${count.format(reason.markings)} markings. A Proved needs both explorers to agree, so this is Unknown.`,
        "Lower capacities or initial tokens, or remove elements from this view.",
      ];
    case "capacityExceededFirst":
      return [
        `Deadlock-free was decided only up to the first time ${reason.place} exceeds its capacity.`,
      ];
    default:
      return assertNever(reason);
  }
}
