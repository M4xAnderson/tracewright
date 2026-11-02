// A Petri net drawn read-only (flows §4.5): places are circles with their token count as a number
// and "capacity 3 · block" under the name, transitions are bars with their name, ports carry a
// notched outline and a "port" tag. Laid out by the layout worker, once per net.
import {
  BaseEdge,
  type EdgeProps,
  type EdgeTypes,
  getBezierPath,
  Handle,
  MarkerType,
  type NodeProps,
  type NodeTypes,
  Position,
  ReactFlow,
  useReactFlow,
} from "@xyflow/react";
import { type CSSProperties, memo, useEffect, useEffectEvent, useMemo, useState } from "react";
import type { PetriNetView } from "../../ipc/bindings/petri-net-view";
import { cn } from "../../lib/cn";
import { type LayoutResult, useLayoutClient } from "../canvas/layout-client";
import {
  type ArcEdge,
  arcs,
  netLayoutRequest,
  netNodes,
  type NetNode,
  placeLabel,
} from "./net-model";

type PlaceNode = Extract<NetNode, { type: "place" }>;
type TransitionNode = Extract<NetNode, { type: "transition" }>;

// Arcs meet the circle or bar, not the label box around it.
const PLACE_RADIUS = 18;
const TRANSITION_HALF_WIDTH = 6;

function ShapeHandles({ halfWidth }: { halfWidth: number }) {
  return (
    <>
      <Handle
        type="target"
        position={Position.Left}
        isConnectable={false}
        className="net-handle"
        style={{ left: `calc(50% - ${halfWidth}px)` }}
      />
      <Handle
        type="source"
        position={Position.Right}
        isConnectable={false}
        className="net-handle"
        style={{ left: `calc(50% + ${halfWidth}px)`, right: "auto" }}
      />
    </>
  );
}

const PlaceItem = memo(function PlaceItem({ data }: NodeProps<PlaceNode>) {
  const { place } = data;
  return (
    <div className="net-item" data-place={place.name}>
      <span className={cn("net-place", place.port && "is-port")}>{place.initial}</span>
      <span className="net-name">{place.displayName}</span>
      <span className="net-detail">{placeLabel(place)}</span>
      {place.port && <span className="net-port-tag">port</span>}
      <ShapeHandles halfWidth={PLACE_RADIUS} />
    </div>
  );
});

const TransitionItem = memo(function TransitionItem({ data }: NodeProps<TransitionNode>) {
  const { transition } = data;
  return (
    <div className="net-item" data-transition={transition.name}>
      <span className={cn("net-transition", transition.port && "is-port")} />
      <span className="net-name">{transition.displayName}</span>
      {transition.port && <span className="net-port-tag">port</span>}
      <ShapeHandles halfWidth={TRANSITION_HALF_WIDTH} />
    </div>
  );
});

const BACK_ARC_DEPTH = 130;

function backArc(sx: number, sy: number, tx: number, ty: number): [string, number, number] {
  const low = Math.max(sy, ty) + BACK_ARC_DEPTH;
  const path = `M${sx},${sy} C${sx + 60},${low} ${tx - 60},${low} ${tx},${ty}`;
  return [path, (sx + tx) / 2, low - BACK_ARC_DEPTH / 4];
}

const Arc = memo(function Arc({
  id,
  data,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  markerEnd,
}: EdgeProps<ArcEdge>) {
  // An arc back to an earlier item loops below the row, so a cycle reads as one.
  const backwards = targetX < sourceX;
  const [path, labelX, labelY] = backwards
    ? backArc(sourceX, sourceY, targetX, targetY)
    : getBezierPath({ sourceX, sourceY, sourcePosition, targetX, targetY, targetPosition });
  const weight = data?.weight ?? 1;
  return (
    <BaseEdge
      id={id}
      path={path}
      markerEnd={markerEnd}
      className="net-arc"
      // Weight 1 is the default and stays unlabeled, as in the files.
      label={weight === 1 ? undefined : weight}
      labelX={labelX}
      labelY={labelY}
      labelBgStyle={{ fill: "var(--surface-panel)" }}
      labelStyle={{ fill: "var(--text-secondary)" }}
    />
  );
});

const nodeTypes = { place: PlaceItem, transition: TransitionItem } satisfies NodeTypes;
const edgeTypes = { arc: Arc } satisfies EdgeTypes;
const defaultEdgeOptions = { markerEnd: { type: MarkerType.ArrowClosed, color: "var(--edge)" } };
// Names stay readable at the first fit; a wider net is panned (flows §4.5, large).
const fitViewOptions = { minZoom: 0.75, maxZoom: 1 };
const ariaLabelConfig = {
  "node.a11yDescription.default": "",
  "node.a11yDescription.keyboardDisabled": "",
  "edge.a11yDescription.default": "",
};

export function NetGraph({ net, elementName }: { net: PetriNetView; elementName: string }) {
  const layout = useLayoutClient();
  const reactFlow = useReactFlow();
  const [laidOut, setLaidOut] = useState<{ net: PetriNetView; result: LayoutResult } | null>(null);

  const handleLaidOut = useEffectEvent((laidOutNet: PetriNetView, result: LayoutResult) => {
    setLaidOut({ net: laidOutNet, result });
    void reactFlow.fitView(fitViewOptions);
  });

  // Synchronizes with the layout worker: one request per net.
  useEffect(() => {
    if (layout === null) {
      return;
    }
    let current = true;
    layout.layout(netLayoutRequest(net)).then(
      (result) => {
        if (current) {
          handleLaidOut(net, result);
        }
      },
      // A failed layout keeps "Laying out…"; the net stays readable in the element's file.
      () => {},
    );
    return () => {
      current = false;
    };
  }, [net, layout]);

  const ready = laidOut !== null && laidOut.net === net;
  const positions = ready ? laidOut.result.positions : null;
  const nodes = useMemo(
    () => (positions === null ? [] : netNodes(net, positions)),
    [net, positions],
  );
  const edges = useMemo(() => (positions === null ? [] : arcs(net)), [net, positions]);

  return (
    <figure aria-label={`Petri net of ${elementName}`} className="net-graph h-full">
      {!ready && (
        <p
          className="appear-after p-3 text-text-muted"
          style={{ "--appear-after": "300ms" } as CSSProperties}
        >
          Laying out…
        </p>
      )}
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        defaultEdgeOptions={defaultEdgeOptions}
        nodesDraggable={false}
        nodesConnectable={false}
        edgesFocusable={false}
        elementsSelectable={false}
        disableKeyboardA11y
        ariaLabelConfig={ariaLabelConfig}
        fitView
        fitViewOptions={fitViewOptions}
        minZoom={0.1}
        proOptions={{ hideAttribution: true }}
      />
    </figure>
  );
}
