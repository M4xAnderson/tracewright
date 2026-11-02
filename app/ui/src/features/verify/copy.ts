// Verdict copy from the flows §5 catalog, word for word. Names are resolved from the view at
// render time (design §3), so a rename never shows an old label.
import type { OverflowDto } from "../../ipc/bindings/overflow-dto";
import type { Phase } from "../../ipc/bindings/phase";
import type { PlaceOrigin } from "../../ipc/bindings/place-origin";
import type { PropertyKind } from "../../ipc/bindings/property-kind";
import type { PropertyView } from "../../ipc/bindings/property-view";
import type { TransitionOrigin } from "../../ipc/bindings/transition-origin";
import type { UnknownDto } from "../../ipc/bindings/unknown-dto";
import type { View } from "../../ipc/bindings/view";
import type { WaitDto } from "../../ipc/bindings/wait-dto";
import type { WaitingDto } from "../../ipc/bindings/waiting-dto";
import { assertNever } from "../../store/app-store";

const count = new Intl.NumberFormat("en-US");

// V-RUN with contract R11. Both counts are indeterminate: the second explorer counts its own
// composition, so the first count is not its denominator (flows §4.7).
export function progressLine(progress: { phase: Phase; markings: number }): string {
  const markings = `${count.format(progress.markings)} ${progress.markings === 1 ? "marking" : "markings"}`;
  switch (progress.phase) {
    case "main":
      return `Exploring… ${markings}`;
    case "recheck":
      return `Re-checking… ${markings}`;
    default:
      return assertNever(progress.phase);
  }
}

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

// V-PRO-T1 / V-PRO-T2
export function provedHeadline(kind: PropertyKind, scope: string): string | null {
  switch (kind) {
    case "deadlockFree":
      return `${scope} never deadlocks.`;
    case "noCapacityExceeded":
      return `${scope} never goes over any capacity.`;
    default:
      return null;
  }
}

// V-DNM-T1 / V-DNM-T2
export function doesNotMean(kind: PropertyKind): string | null {
  switch (kind) {
    case "deadlockFree":
      return "No deadlock does not mean done: a timeout or retry loop can run forever.";
    case "noCapacityExceeded":
      return "Places that block or drop can still make senders wait or lose messages.";
    default:
      return null;
  }
}

// V-PRO-1, exhaustive
export function exhaustiveLine(markings: number): string {
  return `Checked all ${count.format(markings)} reachable markings; re-checked by a second explorer.`;
}

// V-PRO-1, certificate
export const CERTIFICATE_LINE =
  "Proved with a certificate; a second explorer re-checked the certificate.";

// The NS §9.6 fact of a NoOverflowPlace certificate (catalog T2), so "never overflows" is not read
// as "never loses a token". The flows catalog has no ID for it yet; the wording is NS §9.6's, and
// the line for no drop place is ours.
export function certificateFact(view: View, scope: string, dropPlaces: PlaceOrigin[]): string[] {
  const discarded =
    dropPlaces.length === 0
      ? "No place drops, so no token is discarded."
      : `Tokens may be discarded at: ${dropPlaces.map((place) => placeName(view, place)).join(", ")}.`;
  return [`No place in ${scope} can overflow: every place blocks or drops.`, discarded];
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

// A place in the waiting element is bare ("lock_b"); any other is qualified ("Billing: lock_b");
// a generated place is its relationship ("Order API → Billing (publishes to)").
function placeLabel(view: View, place: PlaceOrigin, owner: string | null): string {
  switch (place.kind) {
    case "own":
      return place.element === owner
        ? place.place
        : `${elementName(view, place.element)}: ${place.place}`;
    case "generated":
      return relationshipLabel(view, place.relationship);
    default:
      return assertNever(place);
  }
}

// A generated place holds messages, a place of a behavior model tokens (flows §5).
function unit(place: PlaceOrigin): string {
  return place.kind === "generated" ? "messages" : "tokens";
}

// The place as the copy names it on its own: "Order API → Billing (publishes to)" or
// "Billing: queue".
function placeName(view: View, place: PlaceOrigin): string {
  return placeLabel(view, place, null);
}

// V-DIS-CAP-REL / V-DIS-CAP-PLACE
export function overflowLine(view: View, overflow: OverflowDto): string {
  return `${placeName(view, overflow.place)} holds ${count.format(overflow.count)} ${unit(overflow.place)}; capacity ${count.format(overflow.capacity)}.`;
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
      return `for room in ${place}: ${reason.holds} of ${reason.capacity} ${unit(wait.place)}, when full: block`;
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

// U-CEF-REL / U-CEF-PLACE: what was decided, and the next step. A place inside a behavior model
// is edited in its element's file (file-format §2: elements/<id>.toml) until slice 1 brings the
// behavior model pane.
function capacityExceededFirstLine(
  view: View,
  kind: PropertyKind,
  place: PlaceOrigin,
  capacity: number,
): string {
  const decided = `${propertyName({ kind, template: null })} was decided only up to the first time ${placeName(view, place)} exceeds capacity ${count.format(capacity)}.`;
  switch (place.kind) {
    case "generated":
      return `${decided} Set when full to block or drop for this relationship, or raise its capacity.`;
    case "own":
      return `${decided} Set when full to block or drop for ${place.place} in elements/${place.element}.toml, or raise its capacity, then choose Reload.`;
    default:
      return assertNever(place);
  }
}

// U-INC
export const MODEL_INCOMPLETE =
  "Model incomplete: this view cannot be checked until these are fixed:";

// Flows §4.9: the reason in the user's terms. Lines the engine sends (details) follow the first.
export function unknownLines(view: View, kind: PropertyKind, reason: UnknownDto): string[] {
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
      return [capacityExceededFirstLine(view, kind, reason.place, reason.capacity)];
    default:
      return assertNever(reason);
  }
}
