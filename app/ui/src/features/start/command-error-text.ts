import type { CommandError } from "../../ipc/bindings/command-error";
import { assertNever } from "../../store/app-store";

// Flows §5: P-OPEN-ERR-* for a folder that does not open; E-INT for a bug in the tool.
export function commandErrorText(error: CommandError): string {
  switch (error.kind) {
    case "notAProject":
      return `${error.path} is not a project: arch.toml is missing. Choose a project folder or create a new project.`;
    case "permissionDenied":
      return `Could not open ${error.path}: you do not have permission to read it. Choose another folder or change its permissions.`;
    case "newerFormat":
      return `${error.path} was saved by a newer version of this app (format ${error.format}). Update the app to open it.`;
    case "openFailed":
      return `Could not open ${error.path}: ${error.cause}. Check that the folder is readable, then open it again.`;
    case "invalidArgument":
    case "noProjectOpen":
    case "internal":
      return "Open project stopped because of a bug in the tool, not in your design. Details were saved to the local log.";
    default:
      return assertNever(error);
  }
}
