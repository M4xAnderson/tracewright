// Reload outcomes in the status bar (flows §5: S-RELOAD-OK, S-RELOAD-NONE, S-RELOAD-PARSE), word
// for word. Elements are named through the view Reload returned; a removed one keeps its ID.
import type { ReloadOutcome } from "../../ipc/bindings/reload-outcome";
import type { View } from "../../ipc/bindings/view";
import { assertNever } from "../../store/app-store";
import { elementName } from "../verify/copy";

// "Billing", "Billing and Shipping", "Billing, Shipping and Audit".
function names(list: string[]): string {
  if (list.length <= 1) {
    return list.join("");
  }
  return `${list.slice(0, -1).join(", ")} and ${list.at(-1)}`;
}

export function reloadText(view: View, outcome: ReloadOutcome): string {
  switch (outcome.kind) {
    case "nothingChanged":
      return "No files changed on disk.";
    case "reloaded": {
      const [notLoaded] = outcome.notLoaded;
      if (notLoaded !== undefined) {
        const line = notLoaded.line === null ? "" : `line ${notLoaded.line}: `;
        return `${notLoaded.file ?? "A project file"} was not loaded: ${line}${notLoaded.message}. The app keeps the last version that loaded.`;
      }
      const parts = ["Reloaded."];
      const changed = outcome.changed.map((id) => elementName(view, id));
      if (changed.length > 3) {
        parts.push(`${changed.length} elements changed.`);
      } else if (changed.length > 0) {
        parts.push(`Changed: ${changed.join(", ")}.`);
      }
      const added = outcome.added.map((id) => elementName(view, id));
      if (added.length > 0) {
        const placed = added.length === 1 ? "it" : "them";
        parts.push(`Added ${names(added)} and placed ${placed} automatically.`);
      }
      if (outcome.removed.length > 0) {
        parts.push(`Removed ${names(outcome.removed)}.`);
      }
      return parts.join(" ");
    }
    default:
      return assertNever(outcome);
  }
}
