// The canvas (flows §4.3, read-only until the editing ticket): the open view's elements and
// relationships on React Flow with the E4 settings. Only visible elements render, the zoom
// reaches the nodes as one class on the wrapper, the projection keeps unchanged objects, and
// elkjs places unplaced elements in its worker, once per view.
import {
  type EdgeTypes,
  MarkerType,
  type NodeTypes,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  useStore,
} from "@xyflow/react";
import { useCallback, useEffect, useEffectEvent, useRef, useState } from "react";
import type { View } from "../../ipc/bindings/view";
import { cn } from "../../lib/cn";
import { select } from "../../store/view-actions";
import {
  type CanvasProjection,
  type Placed,
  placementFromLayout,
  projectCanvas,
  unplacedLayoutRequest,
} from "./canvas-model";
import { ElementNode } from "./element-node";
import { GoToDialog } from "./go-to-dialog";
import { type LayoutResult, useLayoutClient } from "./layout-client";
import { RelationshipEdge } from "./relationship-edge";

const nodeTypes = { element: ElementNode } satisfies NodeTypes;
const edgeTypes = { relationship: RelationshipEdge } satisfies EdgeTypes;
const defaultEdgeOptions = { markerEnd: { type: MarkerType.ArrowClosed, color: "var(--edge)" } };

// React Flow's own texts say "node" and "edge" (glossary Avoid words) and describe gestures that
// are off; the canvas describes its elements itself.
const ariaLabelConfig = {
  "node.a11yDescription.default": "",
  "node.a11yDescription.keyboardDisabled": "",
  "edge.a11yDescription.default": "",
  "controls.ariaLabel": "Zoom",
  "controls.zoomIn.ariaLabel": "Zoom in",
  "controls.zoomOut.ariaLabel": "Zoom out",
  "controls.fitView.ariaLabel": "Fit view",
  "handle.ariaLabel": "Port",
};

// The E4 lever: port names and other detail appear at canvas zoom 100 % and above.
const DETAIL_ZOOM = 1;
// Frames to wait for a panned-to element to render before focusing it.
const FOCUS_FRAMES_MAX = 10;

export function Canvas({ view }: { view: View }) {
  return (
    <ReactFlowProvider>
      <CanvasFlow view={view} />
    </ReactFlowProvider>
  );
}

function CanvasFlow({ view }: { view: View }) {
  const layout = useLayoutClient();
  const reactFlow = useReactFlow();
  const wrapper = useRef<HTMLDivElement>(null);
  const detailed = useStore((s) => s.transform[2] >= DETAIL_ZOOM);
  const [placed, setPlaced] = useState<Placed>({});
  const [goTo, setGoTo] = useState<"element" | "relationship" | null>(null);

  // The projection remembers the previous one, so unchanged elements keep their objects. It is
  // derived when the view or the placement changes (React's "adjust state on prop change").
  const [derived, setDerived] = useState<{
    view: View;
    placed: Placed;
    projection: CanvasProjection;
  }>(() => ({ view, placed, projection: projectCanvas(null, view, placed) }));
  let projection = derived.projection;
  if (derived.view !== view || derived.placed !== placed) {
    projection = projectCanvas(derived.projection, view, placed);
    setDerived({ view, placed, projection });
  }

  const handleLaidOut = useEffectEvent((laidOutView: View, result: LayoutResult) => {
    setPlaced(placementFromLayout(laidOutView, result));
    void reactFlow.fitView();
  });

  // Synchronizes with the layout worker: one request per view, for its unplaced elements only.
  useEffect(() => {
    const request = unplacedLayoutRequest(view);
    if (layout === null || request === null) {
      return;
    }
    let current = true;
    layout.layout(request).then(
      (result) => {
        if (current) {
          handleLaidOut(view, result);
        }
      },
      // A failed layout leaves those elements unplaced; the rest of the view stays usable.
      () => {},
    );
    return () => {
      current = false;
    };
  }, [view, layout]);

  const focusElement = useCallback(
    (id: string) => {
      const node = reactFlow.getNode(id);
      const box = wrapper.current;
      if (node === undefined || box === null) {
        return;
      }
      const { x, y, zoom } = reactFlow.getViewport();
      const width = node.width ?? 0;
      const height = node.height ?? 0;
      const left = node.position.x * zoom + x;
      const top = node.position.y * zoom + y;
      const inView =
        left >= 0 &&
        top >= 0 &&
        left + width * zoom <= box.offsetWidth &&
        top + height * zoom <= box.offsetHeight;
      if (!inView) {
        // Instant: viewport pans never animate (AC-18).
        void reactFlow.setCenter(node.position.x + width / 2, node.position.y + height / 2, {
          zoom,
          duration: 0,
        });
      }
      // With onlyRenderVisibleElements the element renders only after the pan.
      let frames = 0;
      const tryFocus = () => {
        const target = box.querySelector<HTMLElement>(`[data-element-id="${CSS.escape(id)}"]`);
        if (target !== null) {
          target.focus({ preventScroll: true });
        } else if (frames < FOCUS_FRAMES_MAX) {
          frames += 1;
          requestAnimationFrame(tryFocus);
        }
      };
      tryFocus();
    },
    [reactFlow],
  );

  // Go to element (Ctrl+P) and Go to relationship (Ctrl+Shift+O) while a project is open.
  useEffect(() => {
    function handleShortcut(event: KeyboardEvent) {
      if (!event.ctrlKey || event.altKey || event.metaKey) {
        return;
      }
      const key = event.key.toLowerCase();
      if (key === "p" && !event.shiftKey) {
        event.preventDefault();
        setGoTo("element");
      } else if (key === "o" && event.shiftKey) {
        event.preventDefault();
        setGoTo("relationship");
      }
    }
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  // The canvas tab ring (flows §2.4): Tab and Shift+Tab move between elements in reading order,
  // including those outside the viewport; past either end, focus leaves the canvas as usual.
  // Escape clears the selection. Delegated from the element buttons, so one listener serves all.
  const handleCanvasKey = useEffectEvent((event: KeyboardEvent) => {
    if (event.key === "Escape") {
      select(null);
      return;
    }
    if (event.key !== "Tab" || !(event.target instanceof HTMLElement)) {
      return;
    }
    const id = event.target.dataset["elementId"];
    if (id === undefined) {
      return;
    }
    const order = projection.nodes.map((n) => n.id);
    const next = order[order.indexOf(id) + (event.shiftKey ? -1 : 1)];
    if (next !== undefined) {
      event.preventDefault();
      focusElement(next);
    }
  });

  useEffect(() => {
    const box = wrapper.current;
    if (box === null) {
      return;
    }
    const listener = (event: KeyboardEvent) => handleCanvasKey(event);
    box.addEventListener("keydown", listener);
    return () => box.removeEventListener("keydown", listener);
  }, []);

  const elementItems = projection.nodes.map((n) => ({ value: n.id, label: n.data.name }));
  const relationshipItems = projection.edges.map((e) => ({
    value: e.id,
    label: e.ariaLabel ?? e.id,
  }));

  return (
    <div ref={wrapper} className={cn("canvas h-full w-full", detailed && "zoom-detail")}>
      <ReactFlow
        nodes={projection.nodes}
        edges={projection.edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        defaultEdgeOptions={defaultEdgeOptions}
        onlyRenderVisibleElements
        nodesDraggable={false}
        nodesConnectable={false}
        nodesFocusable={false}
        edgesFocusable={false}
        elementsSelectable={false}
        disableKeyboardA11y
        ariaLabelConfig={ariaLabelConfig}
        onEdgeClick={(_, edge) => select({ kind: "relationship", id: edge.id })}
        onPaneClick={() => select(null)}
        fitView
        minZoom={0.1}
        // The attribution is a remote link; the app opens no remote URLs (SEC-1).
        proOptions={{ hideAttribution: true }}
      />
      <GoToDialog
        title="Go to element"
        items={elementItems}
        open={goTo === "element"}
        onOpenChange={(open) => setGoTo(open ? "element" : null)}
        onChoose={(id) => {
          setGoTo(null);
          select({ kind: "element", id });
          focusElement(id);
        }}
      />
      <GoToDialog
        title="Go to relationship"
        items={relationshipItems}
        open={goTo === "relationship"}
        onOpenChange={(open) => setGoTo(open ? "relationship" : null)}
        onChoose={(id) => {
          setGoTo(null);
          select({ kind: "relationship", id });
        }}
      />
    </div>
  );
}
