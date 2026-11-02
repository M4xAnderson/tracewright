// One relationship on the canvas (flows §4.3): a line from port handle to port handle, or border to
// border when it binds no ports; dashed when asynchronous; labeled with its meaning and, for an
// asynchronous binding, the chip "capacity 3 · overflow".
import { BaseEdge, EdgeLabelRenderer, type EdgeProps, getSmoothStepPath } from "@xyflow/react";
import { memo } from "react";
import { cn } from "../../lib/cn";
import { useAppStore } from "../../store/app-store";
import type { RelationshipEdge as RelationshipEdgeType } from "./canvas-model";

export const RelationshipEdge = memo(function RelationshipEdge({
  id,
  data,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  markerEnd,
}: EdgeProps<RelationshipEdgeType>) {
  const selected = useAppStore(
    (s) => s.selection?.kind === "relationship" && s.selection.id === id,
  );
  const [path, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
    borderRadius: 4,
  });
  const label = data?.label ?? null;
  const chip = data?.chip ?? null;
  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={markerEnd}
        className={cn(
          "relationship-line",
          data?.asynchronous === true && "is-async",
          selected && "is-selected",
        )}
      />
      {(label !== null || chip !== null) && (
        <EdgeLabelRenderer>
          <div
            aria-hidden="true"
            className="relationship-label"
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
          >
            {label}
            {chip !== null && <span className="relationship-chip">{chip}</span>}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
});
