import { FileWarning } from "lucide-react";
import type { View } from "../../ipc/bindings/view";
import { bannerText, filesNotLoaded } from "../problems/copy";
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
      <FilesNotLoaded view={view} />
      {view.properties.map((property) => (
        <VerdictRow key={property.id} view={view} property={property} />
      ))}
    </section>
  );
}

// V-BANNER-FNL above the rows: some files did not load, so the verdicts cover what did.
function FilesNotLoaded({ view }: { view: View }) {
  const files = filesNotLoaded(view);
  if (files.length === 0) {
    return null;
  }
  return (
    <div
      role="note"
      className="mx-4 mb-2 flex gap-2 rounded border border-border-control bg-surface-sunken px-3 py-2"
    >
      <FileWarning aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-text-muted" />
      <p className="max-w-[75ch]">{bannerText(files)}</p>
    </div>
  );
}
