// One element on the canvas (flows §4.3, 2b mockup): kind icon and label, name, technology, the
// behavior model mark or the "No behavior model" badge, and one handle per port (§4.4). Reads only
// its own data plus whether it is selected (R-4); zoom reaches it only as the wrapper's class.
import { Handle, type NodeProps, Position } from "@xyflow/react";
import { Box, Cylinder, Layers, Network, User } from "lucide-react";
import { memo, useId } from "react";
import type { ElementKind } from "../../ipc/bindings/element-kind";
import { cn } from "../../lib/cn";
import { assertNever, useAppStore } from "../../store/app-store";
import { select } from "../../store/view-actions";
import { BORDER_HANDLE, type ElementNode as ElementNodeType, portHandleId } from "./canvas-model";

export function kindLabel(kind: ElementKind): string {
  switch (kind) {
    case "person":
      return "Person";
    case "softwareSystem":
      return "Software system";
    case "container":
      return "Container";
    case "database":
      return "Database";
    case "queue":
      return "Queue";
    case "component":
      return "Component";
    case "code":
      return "Code element";
    default:
      return assertNever(kind);
  }
}

function KindIcon({ kind }: { kind: ElementKind }) {
  switch (kind) {
    case "person":
      return <User aria-hidden="true" />;
    case "database":
      return <Cylinder aria-hidden="true" />;
    case "queue":
      return <Layers aria-hidden="true" />;
    case "softwareSystem":
    case "container":
    case "component":
    case "code":
      return <Box aria-hidden="true" />;
    default:
      return assertNever(kind);
  }
}

export const ElementNode = memo(function ElementNode({ id, data }: NodeProps<ElementNodeType>) {
  const selected = useAppStore((s) => s.selection?.kind === "element" && s.selection.id === id);
  const descriptionId = useId();
  const kind = kindLabel(data.kind);
  let description = "No behavior model.";
  if (data.hasBehavior) {
    description =
      data.ports.length === 0 ? "No ports." : `Ports: ${data.ports.map((p) => p.name).join(", ")}.`;
  }
  return (
    <div className={cn("element-node", `kind-${data.kind}`, selected && "is-selected")}>
      <button
        type="button"
        data-element-id={id}
        aria-label={`${data.name}, ${kind.toLowerCase()}`}
        aria-describedby={descriptionId}
        aria-pressed={selected}
        onClick={() => select({ kind: "element", id })}
        className="element-body nodrag nopan"
        // React Flow turns pointer events off on wrappers it handles no clicks for.
        style={{ pointerEvents: "auto" }}
      >
        <span className="element-glyph">
          <KindIcon kind={data.kind} />
        </span>
        <span className="element-name">{data.name}</span>
        {data.hasBehavior ? (
          <Network aria-hidden="true" className="element-mark" />
        ) : (
          <span className="element-badge">No behavior model</span>
        )}
        <span className="element-tech">
          {kind}
          {data.technology !== null && ` · ${data.technology}`}
        </span>
      </button>
      <span id={descriptionId} hidden>
        {description}
      </span>
      {data.ports.map((port) => {
        const position = port.side === "left" ? Position.Left : Position.Right;
        const style = { top: port.offset };
        return (
          <div key={port.name}>
            <Handle
              id={portHandleId(port.name)}
              type="source"
              position={position}
              isConnectable={false}
              className={cn("port-handle", `port-${port.kind}`)}
              style={style}
            >
              <span className={cn("port-name", `port-name-${port.side}`)}>{port.name}</span>
            </Handle>
            <Handle
              id={portHandleId(port.name)}
              type="target"
              position={position}
              isConnectable={false}
              className="port-handle port-twin"
              style={style}
            />
          </div>
        );
      })}
      <Handle
        id={BORDER_HANDLE}
        type="target"
        position={Position.Left}
        isConnectable={false}
        className="border-handle"
      />
      <Handle
        id={BORDER_HANDLE}
        type="source"
        position={Position.Right}
        isConnectable={false}
        className="border-handle"
      />
    </div>
  );
});
