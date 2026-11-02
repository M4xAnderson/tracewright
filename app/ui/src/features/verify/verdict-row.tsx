// One row of the verdict panel in the fixed anatomy of flows §4.8: property name; scope and
// level; verdict line (icon plus word); detail.
import { CircleCheck, CircleHelp, CircleX, LoaderCircle } from "lucide-react";
import { type CSSProperties, useId } from "react";
import { useShallow } from "zustand/react/shallow";
import type { DisprovedDto } from "../../ipc/bindings/disproved-dto";
import type { PropertyKind } from "../../ipc/bindings/property-kind";
import type { PropertyView } from "../../ipc/bindings/property-view";
import type { ProvedDto } from "../../ipc/bindings/proved-dto";
import type { View } from "../../ipc/bindings/view";
import type { UnknownDto } from "../../ipc/bindings/unknown-dto";
import { assertNever, useAppStore } from "../../store/app-store";
import { incompleteLines } from "../problems/copy";
import {
  assumptionLine,
  CERTIFICATE_LINE,
  deadlockHeadline,
  doesNotMean,
  elementName,
  exhaustiveLine,
  NO_FINAL_S0,
  overflowLine,
  progressLine,
  propertyName,
  provedHeadline,
  SCOPE_LINE,
  scopeLine,
  stepText,
  unknownLines,
  waitingLine,
} from "./copy";
import { type RowState, verdictView } from "./verdict-view";

// A deadlock with this many waiting lines scrolls inside its row (flows §4.8, long text).
const WAITING_SCROLL_FROM = 10;

export function VerdictRow({ view, property }: { view: View; property: PropertyView }) {
  const state = useAppStore(useShallow((s) => verdictView(s, property.id)));
  const headingId = useId();
  return (
    <article
      aria-labelledby={headingId}
      aria-busy={state.kind === "pending"}
      className="grid grid-cols-[22px_1fr] gap-x-2 border-t border-border-hairline px-4 pt-3 pb-3.5"
    >
      <div className="col-span-2">
        <h3 id={headingId} className="font-semibold">
          {propertyName(property)}
        </h3>
        <p className="text-text-secondary">{scopeLine(view, property)}</p>
      </div>
      <VerdictLine state={state} />
      <div className="col-start-2 max-w-[75ch] space-y-1 text-text-secondary">
        <Detail view={view} property={property} state={state} />
      </div>
    </article>
  );
}

function VerdictLine({ state }: { state: RowState }) {
  const line = "col-span-2 mt-1 flex h-6 items-center gap-2 font-semibold";
  switch (state.kind) {
    case "pending":
      // The placeholder keeps the row's height; nothing reads as a verdict.
      return (
        <div className={line} aria-hidden="true">
          <span className="size-5 rounded-full border-2 border-dashed border-verdict-pending" />
        </div>
      );
    case "running":
      return (
        <div className={line}>
          <LoaderCircle aria-hidden="true" className="size-5 animate-spin text-text-muted" />
          <span>Checking…</span>
        </div>
      );
    case "noBehavior":
      return <div className={line}>—</div>;
    case "noVerdict":
      return null;
    case "unknownKey":
      return <Word kind="unknown" />;
    case "current":
      return <Word kind={state.verdict.kind} />;
    default:
      return assertNever(state);
  }
}

function Word({ kind }: { kind: "proved" | "disproved" | "unknown" }) {
  const line = "col-span-2 mt-1 flex h-6 items-center gap-2 font-semibold";
  switch (kind) {
    case "proved":
      return (
        <div className={line}>
          <CircleCheck aria-hidden="true" className="size-5 text-verdict-proved" />
          <span>Proved</span>
        </div>
      );
    case "disproved":
      return (
        <div className={line}>
          <CircleX aria-hidden="true" className="size-5 text-verdict-disproved" />
          <span>Disproved</span>
        </div>
      );
    case "unknown":
      return (
        <div className={line}>
          <CircleHelp aria-hidden="true" className="size-5 text-verdict-unknown" />
          <span>Unknown</span>
        </div>
      );
    default:
      return assertNever(kind);
  }
}

// A model-incomplete reason names its causes in the user's terms (U-INC-*, flows R2).
function reasonLines(view: View, kind: PropertyKind, reason: UnknownDto): string[] {
  if (reason.category === "modelIncomplete" && reason.causes.length > 0) {
    // Lines after the causes are what only lowering found; they have no problem kind.
    return [...incompleteLines(view, reason.causes), ...reason.details.slice(reason.causes.length)];
  }
  return unknownLines(view, kind, reason);
}

function Lines({ lines }: { lines: string[] }) {
  return (
    <>
      {lines.map((text) => (
        <p key={text}>{text}</p>
      ))}
    </>
  );
}

function Detail({
  view,
  property,
  state,
}: {
  view: View;
  property: PropertyView;
  state: RowState;
}) {
  switch (state.kind) {
    case "pending":
      return (
        <p className="appear-after" style={{ "--appear-after": "1s" } as CSSProperties}>
          Updating checks…
        </p>
      );
    case "running":
      return state.progress === undefined ? null : <p>{progressLine(state.progress)}</p>;
    case "noBehavior":
      return <p>Nothing to check yet: no element in this view has a behavior model.</p>;
    case "noVerdict":
      return (
        <p>
          {state.behaviorChanged ? "Behavior changed since the last check." : "Not checked yet."}
        </p>
      );
    case "unknownKey":
      return <Lines lines={reasonLines(view, property.kind, state.reason)} />;
    case "current": {
      const { verdict } = state;
      const assumptions = verdict.assumptions.map((r) => assumptionLine(view, r));
      switch (verdict.kind) {
        case "proved":
          return (
            <ProvedDetail
              view={view}
              property={property}
              proved={verdict}
              assumptions={assumptions}
            />
          );
        case "disproved":
          return (
            <DisprovedDetail
              view={view}
              property={property}
              disproved={verdict}
              assumptions={assumptions}
            />
          );
        case "unknown":
          return (
            <Lines lines={[...reasonLines(view, property.kind, verdict.reason), ...assumptions]} />
          );
        default:
          return assertNever(verdict);
      }
    }
    default:
      return assertNever(state);
  }
}

function ProvedDetail(props: {
  view: View;
  property: PropertyView;
  proved: ProvedDto;
  assumptions: string[];
}) {
  const { view, property, proved, assumptions } = props;
  const scope = elementName(view, property.scope.element);
  const headline = provedHeadline(property.kind, scope);
  const limits = doesNotMean(property.kind);
  return (
    <>
      {headline !== null && <p className="text-text-primary">{headline}</p>}
      {limits !== null && <p>{limits}</p>}
      <p>
        {proved.technique === "exhaustive" ? exhaustiveLine(proved.markings) : CERTIFICATE_LINE}
      </p>
      <Lines lines={assumptions} />
      <p>{SCOPE_LINE}</p>
    </>
  );
}

function DisprovedDetail(props: {
  view: View;
  property: PropertyView;
  disproved: DisprovedDto;
  assumptions: string[];
}) {
  const { view, property, disproved, assumptions } = props;
  const frames = disproved.frames;
  return (
    <>
      <Violation view={view} property={property} disproved={disproved} />
      <Lines lines={assumptions} />
      <p className="pt-1 font-medium text-text-primary">Counterexample</p>
      <ol aria-label="Counterexample steps" className="font-mono text-[12px]">
        {frames.map((frame) => (
          <li key={frame.step}>{stepText(view, frame.step, frames.length, frame.fired)}</li>
        ))}
      </ol>
    </>
  );
}

// The violation sentence of flows §4.8: on deadlock-free V-DIS-T1, the waiting lines and the
// done-when caveat; on no capacity exceeded V-DIS-CAP-*.
function Violation(props: { view: View; property: PropertyView; disproved: DisprovedDto }) {
  const { view, property, disproved } = props;
  const violation = disproved.violation;
  switch (violation.kind) {
    case "overflow":
      return <p className="text-text-primary">{overflowLine(view, violation)}</p>;
    case "deadlock": {
      const scope = elementName(view, property.scope.element);
      const scrolls = violation.waiting.length >= WAITING_SCROLL_FROM;
      return (
        <>
          <p className="text-text-primary">{deadlockHeadline(scope, disproved.frames.length)}</p>
          <ul
            aria-label="Waiting steps"
            // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- a list that scrolls inside its row must be reachable by keyboard (flows §4.8, A11Y-2)
            tabIndex={scrolls ? 0 : undefined}
            className={scrolls ? "max-h-[18em] space-y-1 overflow-y-auto" : "space-y-1"}
          >
            {violation.waiting.map((waiting) => {
              const text = waitingLine(view, waiting);
              return <li key={text}>{text}</li>;
            })}
          </ul>
          {violation.finished && <p>{NO_FINAL_S0}</p>}
        </>
      );
    }
    default:
      return assertNever(violation);
  }
}
