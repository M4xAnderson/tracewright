// The canvas adapter (NAM-1): maps the view's elements and relationships to React Flow nodes and
// edges. The projection keeps every node and edge object whose element or relationship did not
// change, so a rename re-renders one node out of 300 (E4 setting 4).
import type { Edge, Node } from "@xyflow/react";
import type { BehaviorView } from "../../ipc/bindings/behavior-view";
import type { ElementKind } from "../../ipc/bindings/element-kind";
import type { ElementView } from "../../ipc/bindings/element-view";
import type { RelationshipView } from "../../ipc/bindings/relationship-view";
import type { View } from "../../ipc/bindings/view";
import { assertNever } from "../../store/app-store";
import type { LayoutRequest, LayoutResult, Point } from "./layout-client";

export const ELEMENT_WIDTH = 200;
const ELEMENT_HEIGHT_MIN = 64;
// Port handles sit 24 px apart, so their 24 × 24 hit areas never overlap (flows §4.4, AC-16).
const PORT_PITCH = 24;
// The first port sits below the name and kind lines, so port names fit inside the element.
const PORT_FIRST = 60;
const PORT_LAST_MARGIN = 20;
// Room between layers for relationship lines and their labels.
const LAYER_GAP = 120;
// Unplaced elements are laid out below everything already placed, so nothing placed moves.
const PLACEMENT_GAP = 80;

export type PortSide = "left" | "right";

export interface PortHandle {
  name: string;
  kind: "transition" | "place";
  side: PortSide;
  // The handle's center, from the top of the element.
  offset: number;
}

// Whether the element has a behavior model, and whether it loaded (flows §4.3 badges).
export type BehaviorState = "petriNet" | "notLoaded" | "none";

export interface ElementData extends Record<string, unknown> {
  name: string;
  kind: ElementKind;
  technology: string | null;
  behavior: BehaviorState;
  ports: readonly PortHandle[];
}

export type ElementNode = Node<ElementData, "element">;

export interface RelationshipData extends Record<string, unknown> {
  label: string | null;
  // "capacity 3 · overflow" for an asynchronous binding (flows §4.3).
  chip: string | null;
  asynchronous: boolean;
}

export type RelationshipEdge = Edge<RelationshipData, "relationship">;

export interface CanvasProjection {
  nodes: ElementNode[];
  edges: RelationshipEdge[];
}

// ELK positions of elements whose layout file has none, by element ID.
export type Placed = Readonly<Record<string, Point>>;

export const BORDER_HANDLE = "border";

export function portHandleId(port: string): string {
  return `port:${port}`;
}

// React Flow's wrappers announce "node" and "edge" (glossary Avoid words); ours say what they are.
const ELEMENT_DOM = { "aria-roledescription": undefined };
const RELATIONSHIP_DOM = { "aria-roledescription": "relationship" };

export function projectCanvas(
  previous: CanvasProjection | null,
  view: View,
  placed: Placed,
): CanvasProjection {
  const oldNodes = new Map(previous?.nodes.map((n) => [n.id, n]));
  const oldEdges = new Map(previous?.edges.map((e) => [e.id, e]));
  const nodes: ElementNode[] = [];
  for (const element of view.elements) {
    const position = element.position ?? placed[element.id];
    if (position === undefined) {
      continue;
    }
    const next = elementNode(element, view.relationships, { x: position.x, y: position.y });
    const old = oldNodes.get(element.id);
    nodes.push(old !== undefined && sameNode(old, next) ? old : next);
  }
  const drawn = new Map(nodes.map((n) => [n.id, n]));
  const portsById = new Map(nodes.map((n) => [n.id, n.data.ports]));
  const edges: RelationshipEdge[] = [];
  for (const relationship of view.relationships) {
    const source = drawn.get(relationship.source);
    const target = drawn.get(relationship.target);
    if (source === undefined || target === undefined) {
      continue;
    }
    const next = relationshipEdge(relationship, source.data.name, target.data.name, portsById);
    const old = oldEdges.get(relationship.id);
    edges.push(old !== undefined && sameEdge(old, next) ? old : next);
  }
  const ordered = readingOrder(nodes);
  if (previous !== null && sameItems(previous.nodes, ordered) && sameItems(previous.edges, edges)) {
    return previous;
  }
  return { nodes: ordered, edges };
}

// The view's unplaced elements and the relationships between them, or `null` when every element
// has a position (flows §4.3: only unplaced elements are placed by ELK).
export function unplacedLayoutRequest(view: View): LayoutRequest | null {
  const unplaced = view.elements.filter((e) => e.position === null);
  if (unplaced.length === 0) {
    return null;
  }
  const ids = new Set(unplaced.map((e) => e.id));
  const portsById = new Map(
    unplaced.map((element) => [element.id, elementPorts(element, view.relationships)]),
  );
  return {
    layerGap: LAYER_GAP,
    nodes: unplaced.map((element) => {
      const ports = portsById.get(element.id) ?? [];
      const size = elementSize(element, ports);
      return {
        id: element.id,
        width: size.width,
        height: size.height,
        ports: ports.map((p) => ({
          id: `${element.id}/${p.name}`,
          side: p.side,
          offset: p.offset,
        })),
      };
    }),
    edges: view.relationships
      .filter((r) => ids.has(r.source) && ids.has(r.target))
      .map((relationship) => {
        const handles = attachedPorts(relationship, portsById);
        return {
          id: relationship.id,
          source:
            handles === null ? relationship.source : `${relationship.source}/${handles.source}`,
          target:
            handles === null ? relationship.target : `${relationship.target}/${handles.target}`,
        };
      }),
  };
}

// The layout of the unplaced elements, moved below the placed ones.
export function placementFromLayout(view: View, result: LayoutResult): Placed {
  let left = Number.POSITIVE_INFINITY;
  let bottom = Number.NEGATIVE_INFINITY;
  for (const element of view.elements) {
    if (element.position === null) {
      continue;
    }
    const size = elementSize(element, elementPorts(element, view.relationships));
    left = Math.min(left, element.position.x);
    bottom = Math.max(bottom, element.position.y + size.height);
  }
  const origin =
    bottom === Number.NEGATIVE_INFINITY ? { x: 0, y: 0 } : { x: left, y: bottom + PLACEMENT_GAP };
  // ELK pads the graph; its top-left element goes to the origin.
  const points = Object.values(result.positions);
  const minX = Math.min(...points.map((p) => p.x));
  const minY = Math.min(...points.map((p) => p.y));
  const placed: Record<string, Point> = {};
  for (const [id, point] of Object.entries(result.positions)) {
    placed[id] = {
      x: Math.round(origin.x + point.x - minX),
      y: Math.round(origin.y + point.y - minY),
    };
  }
  return placed;
}

function elementNode(
  element: ElementView,
  relationships: readonly RelationshipView[],
  position: Point,
): ElementNode {
  const ports = elementPorts(element, relationships);
  const size = elementSize(element, ports);
  return {
    id: element.id,
    type: "element",
    position,
    width: size.width,
    height: size.height,
    domAttributes: ELEMENT_DOM,
    data: {
      name: element.name,
      kind: element.kind,
      technology: element.technology,
      behavior: element.behavior === null ? "none" : behaviorState(element.behavior.kind),
      ports,
    },
  };
}

// A port is drawn on the side its relationships arrive from: left when the element is the target
// of a relationship that binds it, right otherwise. Each side lists its ports by name.
function elementPorts(
  element: ElementView,
  relationships: readonly RelationshipView[],
): PortHandle[] {
  if (element.behavior === null || element.behavior.kind !== "petriNet") {
    return [];
  }
  const net = element.behavior;
  const targetPorts = new Set<string>();
  for (const relationship of relationships) {
    if (relationship.target !== element.id) {
      continue;
    }
    for (const binding of relationship.bindings) {
      for (const end of [binding.from, binding.to]) {
        if (end.element === element.id) {
          targetPorts.add(end.name);
        }
      }
    }
  }
  const ports = [
    ...net.transitions
      .filter((t) => t.port)
      .map((t) => ({ name: t.name, kind: "transition" as const })),
    ...net.places.filter((p) => p.port).map((p) => ({ name: p.name, kind: "place" as const })),
  ].sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  const count: Record<PortSide, number> = { left: 0, right: 0 };
  return ports.map((port) => {
    const side: PortSide = targetPorts.has(port.name) ? "left" : "right";
    const offset = PORT_FIRST + count[side] * PORT_PITCH;
    count[side] += 1;
    return { ...port, side, offset };
  });
}

function behaviorState(kind: BehaviorView["kind"]): BehaviorState {
  switch (kind) {
    case "petriNet":
      return "petriNet";
    case "unloaded":
      return "notLoaded";
    default:
      return assertNever(kind);
  }
}

function elementSize(
  element: ElementView,
  ports: readonly PortHandle[],
): { width: number; height: number } {
  const lowest = Math.max(0, ...ports.map((p) => p.offset));
  return {
    width: element.position?.w ?? ELEMENT_WIDTH,
    height:
      element.position?.h ??
      Math.max(ELEMENT_HEIGHT_MIN, lowest === 0 ? 0 : lowest + PORT_LAST_MARGIN),
  };
}

// The ports a relationship attaches to: those of its first binding pair, in the relationship's
// direction (a reply pair runs from target to source). A pair that names a port its element does
// not offer is a model problem (L3, L5); the relationship then attaches to the borders.
function attachedPorts(
  relationship: RelationshipView,
  ports: ReadonlyMap<string, readonly PortHandle[]>,
): { source: string; target: string } | null {
  const pair = relationship.bindings[0];
  if (pair === undefined) {
    return null;
  }
  const attached =
    pair.from.element === relationship.source
      ? { source: pair.from.name, target: pair.to.name }
      : { source: pair.to.name, target: pair.from.name };
  const offers = (element: string, name: string) =>
    ports.get(element)?.some((p) => p.name === name) ?? false;
  if (
    !offers(relationship.source, attached.source) ||
    !offers(relationship.target, attached.target)
  ) {
    return null;
  }
  return attached;
}

function relationshipEdge(
  relationship: RelationshipView,
  sourceName: string,
  targetName: string,
  elementPortsById: ReadonlyMap<string, readonly PortHandle[]>,
): RelationshipEdge {
  const ports = attachedPorts(relationship, elementPortsById);
  const asynchronous = relationship.bindings.find((b) => b.mode.kind === "async");
  let chip: string | null = null;
  if (asynchronous !== undefined && asynchronous.mode.kind === "async") {
    const capacity = asynchronous.mode.capacity ?? "not stated";
    chip = `capacity ${capacity} · ${asynchronous.mode.whenFull}`;
  }
  const meaning = relationship.meaning === null ? "" : ` (${relationship.meaning})`;
  return {
    id: relationship.id,
    type: "relationship",
    source: relationship.source,
    target: relationship.target,
    sourceHandle: ports === null ? BORDER_HANDLE : portHandleId(ports.source),
    targetHandle: ports === null ? BORDER_HANDLE : portHandleId(ports.target),
    ariaLabel: `${sourceName} → ${targetName}${meaning}`,
    domAttributes: RELATIONSHIP_DOM,
    data: { label: relationship.meaning, chip, asynchronous: asynchronous !== undefined },
  };
}

// Reading order for the canvas tab ring (flows §2.4): rows top to bottom, left to right within a
// row. An element joins the row whose first element's vertical middle it starts above.
function readingOrder(nodes: ElementNode[]): ElementNode[] {
  const byTop = [...nodes].sort(
    (a, b) => a.position.y - b.position.y || a.position.x - b.position.x,
  );
  const rows: ElementNode[][] = [];
  let rowEnd = Number.NEGATIVE_INFINITY;
  for (const node of byTop) {
    const row = rows.at(-1);
    if (row !== undefined && node.position.y < rowEnd) {
      row.push(node);
    } else {
      rows.push([node]);
      rowEnd = node.position.y + (node.height ?? ELEMENT_HEIGHT_MIN) / 2;
    }
  }
  return rows.flatMap((row) => row.sort((a, b) => a.position.x - b.position.x));
}

function sameNode(a: ElementNode, b: ElementNode): boolean {
  return (
    a.position.x === b.position.x &&
    a.position.y === b.position.y &&
    a.width === b.width &&
    a.height === b.height &&
    a.data.name === b.data.name &&
    a.data.kind === b.data.kind &&
    a.data.technology === b.data.technology &&
    a.data.behavior === b.data.behavior &&
    a.data.ports.length === b.data.ports.length &&
    a.data.ports.every((port, i) => {
      const other = b.data.ports[i];
      return (
        other !== undefined &&
        port.name === other.name &&
        port.kind === other.kind &&
        port.side === other.side &&
        port.offset === other.offset
      );
    })
  );
}

function sameEdge(a: RelationshipEdge, b: RelationshipEdge): boolean {
  return (
    a.source === b.source &&
    a.target === b.target &&
    a.sourceHandle === b.sourceHandle &&
    a.targetHandle === b.targetHandle &&
    a.ariaLabel === b.ariaLabel &&
    a.data?.label === b.data?.label &&
    a.data?.chip === b.data?.chip &&
    a.data?.asynchronous === b.data?.asynchronous
  );
}

function sameItems<T>(a: readonly T[], b: readonly T[]): boolean {
  return a.length === b.length && a.every((item, i) => item === b[i]);
}
