import type { View } from "../../ipc/bindings/view";
import { VerdictRow } from "./verdict-row";

// The Verdicts tab of the right column (flows §2.1): every property in view, rows in a fixed order.
export function VerdictPanel({ view }: { view: View }) {
  return (
    <section
      aria-label="Verdicts"
      className="flex w-[360px] shrink-0 flex-col overflow-y-auto border-l border-border-hairline bg-surface-panel"
    >
      <h2 className="px-4 py-2.5 text-[11px] font-semibold tracking-wide text-text-muted uppercase">
        Verdicts
      </h2>
      {view.properties.map((property) => (
        <VerdictRow key={property.id} view={view} property={property} />
      ))}
    </section>
  );
}
