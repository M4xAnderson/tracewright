// The behavior model pane's adapter (NAM-1): a Petri net mapped to React Flow nodes for places and
// transitions and edges for arcs, and the layout request that places them. Deterministic, so the
// same net always gets the same layout (flows §4.5).
import type { Edge, Node } from "@xyflow/react";
import type { PetriNetView } from "../../ipc/bindings/petri-net-view";
import type { PlaceView } from "../../ipc/bindings/place-view";
import type { TransitionView } from "../../ipc/bindings/transition-view";
import type { LayoutRequest, Point } from "../canvas/layout-client";

// A place is its circle plus the two label lines under it; a transition its bar plus the name
// above it. Names longer than the width truncate (flows §4.5, large).
const ITEM_WIDTH = 128;
const LAYER_GAP = 40;
const PLACE_HEIGHT = 84;
const TRANSITION_HEIGHT = 72;

export interface PlaceData extends Record<string, unknown> {
  place: PlaceView;
}

export interface TransitionData extends Record<string, unknown> {
  transition: TransitionView;
}

export type NetNode = Node<PlaceData, "place"> | Node<TransitionData, "transition">;

export interface ArcData extends Record<string, unknown> {
  weight: number;
}

export type ArcEdge = Edge<ArcData, "arc">;

const NET_DOM = { "aria-roledescription": undefined };

// Places and transitions share one namespace (NS §2.2), but a name clash still loads (L10), so
// node IDs carry their kind.
function placeId(name: string): string {
  return `place:${name}`;
}

function transitionId(name: string): string {
  return `transition:${name}`;
}

export function netLayoutRequest(net: PetriNetView): LayoutRequest {
  const nodes = [
    ...net.places.map((p) => ({ id: placeId(p.name), height: PLACE_HEIGHT })),
    ...net.transitions.map((t) => ({ id: transitionId(t.name), height: TRANSITION_HEIGHT })),
  ].map((node) => ({ ...node, width: ITEM_WIDTH, ports: [] }));
  return {
    layerGap: LAYER_GAP,
    nodes,
    edges: arcs(net).map(({ id, source, target }) => ({ id, source, target })),
  };
}

export function placeLabel(place: PlaceView): string {
  const capacity = place.capacity === null ? "capacity not stated" : `capacity ${place.capacity}`;
  const whenFull = place.whenFull === null ? "when full not stated" : place.whenFull;
  return `${capacity} · ${whenFull}`;
}

export function netNodes(net: PetriNetView, positions: Readonly<Record<string, Point>>): NetNode[] {
  const at = (id: string) => positions[id] ?? { x: 0, y: 0 };
  const tokens = (n: number) => (n === 1 ? "1 token" : `${n} tokens`);
  return [
    ...net.places.map((place): NetNode => ({
      id: placeId(place.name),
      type: "place",
      position: at(placeId(place.name)),
      width: ITEM_WIDTH,
      height: PLACE_HEIGHT,
      ariaRole: "group",
      ariaLabel: `Place ${place.displayName}${place.port ? ", port" : ""}: ${tokens(place.initial)}, ${placeLabel(place)}`,
      domAttributes: NET_DOM,
      data: { place },
    })),
    ...net.transitions.map((transition): NetNode => ({
      id: transitionId(transition.name),
      type: "transition",
      position: at(transitionId(transition.name)),
      width: ITEM_WIDTH,
      height: TRANSITION_HEIGHT,
      ariaRole: "group",
      ariaLabel: `Transition ${transition.displayName}${transition.port ? ", port" : ""}`,
      domAttributes: NET_DOM,
      data: { transition },
    })),
  ];
}

const ARC_DOM = { "aria-roledescription": "arc" };

// An arc to a place the net does not have is a model problem (L3), listed in Problems; it is not
// drawn.
export function arcs(net: PetriNetView): ArcEdge[] {
  const places = new Set(net.places.map((p) => p.name));
  const known = (arc: { place: string }) => places.has(arc.place);
  return net.transitions.flatMap((transition) => [
    ...transition.pre.filter(known).map((arc): ArcEdge => ({
      id: `pre:${transition.name}:${arc.place}`,
      type: "arc",
      source: placeId(arc.place),
      target: transitionId(transition.name),
      ariaLabel: `${arc.place} → ${transition.name}, weight ${arc.weight}`,
      domAttributes: ARC_DOM,
      data: { weight: arc.weight },
    })),
    ...transition.post.filter(known).map((arc): ArcEdge => ({
      id: `post:${transition.name}:${arc.place}`,
      type: "arc",
      source: transitionId(transition.name),
      target: placeId(arc.place),
      ariaLabel: `${transition.name} → ${arc.place}, weight ${arc.weight}`,
      domAttributes: ARC_DOM,
      data: { weight: arc.weight },
    })),
  ]);
}
