// The behavior model pane, read-only in slice 0 (flows §4.5): the selected element's Petri net, or
// what the selected relationship adds to the combined behavior, or what to do when nothing is
// there to show. Names come from the view at render time.
import { ReactFlowProvider, useReactFlow } from "@xyflow/react";
import { Button } from "../../components/ui/button";
import type { BindingView } from "../../ipc/bindings/binding-view";
import type { ElementView } from "../../ipc/bindings/element-view";
import type { RelationshipView } from "../../ipc/bindings/relationship-view";
import type { View } from "../../ipc/bindings/view";
import { assertNever, useAppStore } from "../../store/app-store";
import { elementName } from "../verify/copy";
import { NetGraph } from "./net-graph";

export function BehaviorPane({ view }: { view: View }) {
  const selection = useAppStore((s) => s.selection);
  let content = <p className="p-3 text-text-muted">Select an element to see its behavior model.</p>;
  if (selection?.kind === "element") {
    const element = view.elements.find((e) => e.id === selection.id);
    if (element !== undefined) {
      content = (
        <ReactFlowProvider key={element.id}>
          <ElementBehavior element={element} />
        </ReactFlowProvider>
      );
    }
  } else if (selection?.kind === "relationship") {
    const relationship = view.relationships.find((r) => r.id === selection.id);
    if (relationship !== undefined) {
      content = <RelationshipBehavior view={view} relationship={relationship} />;
    }
  }
  return (
    <section
      aria-label="Bottom pane: Behavior model"
      className="flex h-[280px] max-h-[40vh] min-h-40 flex-col bg-surface-panel"
    >
      {content}
    </section>
  );
}

function PaneHeader({ title, children }: { title: string; children?: React.ReactNode }) {
  return (
    <div className="flex h-[34px] shrink-0 items-center gap-2.5 border-b border-border-hairline px-3">
      <h2 className="font-semibold">{title}</h2>
      {children}
    </div>
  );
}

function ElementBehavior({ element }: { element: ElementView }) {
  const reactFlow = useReactFlow();
  const title = `Behavior model: ${element.name}`;
  if (element.behavior === null) {
    return (
      <>
        <PaneHeader title={title} />
        <p className="max-w-[75ch] p-3">
          No behavior model. Add it in elements/{element.id}.toml, then choose Reload.
        </p>
      </>
    );
  }
  if (element.behavior.kind === "unloaded") {
    const { line, message } = element.behavior;
    return (
      <>
        <PaneHeader title={title} />
        <p className="max-w-[75ch] p-3">
          The behavior model in elements/{element.id}.toml did not load
          {line === null ? "" : ` (line ${line})`}: {message}. Fix it, then choose Reload.
        </p>
      </>
    );
  }
  return (
    <>
      <PaneHeader title={title}>
        <span className="text-text-muted">
          Petri net, read-only. Edit elements/{element.id}.toml, then Reload.
        </span>
        <span className="flex-1" />
        <Button variant="ghost" onClick={() => void reactFlow.fitView()}>
          Fit
        </Button>
      </PaneHeader>
      <div className="min-h-0 flex-1">
        <NetGraph net={element.behavior} elementName={element.name} />
      </div>
    </>
  );
}

function RelationshipBehavior({
  view,
  relationship,
}: {
  view: View;
  relationship: RelationshipView;
}) {
  const meaning = relationship.meaning === null ? "" : ` (${relationship.meaning})`;
  const title = `${elementName(view, relationship.source)} → ${elementName(view, relationship.target)}${meaning}`;
  return (
    <>
      <PaneHeader title={title} />
      <div className="max-w-[75ch] space-y-1 p-3">
        {relationship.bindings.length === 0 ? (
          <p>
            This relationship binds no ports, so it adds no behavior. Verdicts list it as an
            assumption.
          </p>
        ) : (
          relationship.bindings.map((binding) => (
            <p
              key={`${binding.from.element}/${binding.from.name}>${binding.to.element}/${binding.to.name}`}
            >
              {title} {bindingSentence(view, binding)}
            </p>
          ))
        )}
      </div>
    </>
  );
}

// flows §4.5, relationship selected.
function bindingSentence(view: View, binding: BindingView): string {
  const port = (end: BindingView["from"]) => `${elementName(view, end.element)}: ${end.name}`;
  switch (binding.mode.kind) {
    case "sync":
      return `joins ${port(binding.from)} and ${port(binding.to)} into one step.`;
    case "async": {
      const capacity = binding.mode.capacity ?? "not stated";
      return `adds a place with capacity ${capacity} (when full: ${binding.mode.whenFull}) between ${port(binding.from)} and ${port(binding.to)}.`;
    }
    default:
      return assertNever(binding.mode);
  }
}
