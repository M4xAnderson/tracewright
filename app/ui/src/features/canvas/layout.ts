// elkjs `layered` on one graph (E4 setting 5). Imported only by the layout worker and by tests,
// which run it in process; the app's main thread never loads elkjs (R-4).
import ELK from "elkjs/lib/elk.bundled.js";
import type { ElkExtendedEdge, ElkNode } from "elkjs/lib/elk.bundled.js";
import type { LayoutRequest, LayoutResult } from "./layout-client";

const elk = new ELK();

// Port handles are drawn by the element node at fixed offsets, so ELK gets their exact positions.
const PORT_SIZE = 8;

export async function runLayout(request: LayoutRequest): Promise<LayoutResult> {
  const graph: ElkNode = {
    id: "root",
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": "RIGHT",
      "elk.edgeRouting": "ORTHOGONAL",
      "elk.spacing.nodeNode": "48",
      "elk.layered.spacing.nodeNodeBetweenLayers": "120",
      // Model order keeps the result the same for the same view (flows §4.5: same net, same
      // layout), whatever ELK's internal tie-breaking.
      "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
    },
    children: request.nodes.map((node) => ({
      id: node.id,
      width: node.width,
      height: node.height,
      layoutOptions: { "elk.portConstraints": "FIXED_POS" },
      ports: node.ports.map((port) => ({
        id: port.id,
        width: PORT_SIZE,
        height: PORT_SIZE,
        x: port.side === "left" ? -PORT_SIZE / 2 : node.width - PORT_SIZE / 2,
        y: port.offset - PORT_SIZE / 2,
      })),
    })),
    edges: request.edges.map((edge): ElkExtendedEdge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  };
  const laidOut = await elk.layout(graph);
  const result: LayoutResult = { positions: {} };
  for (const child of laidOut.children ?? []) {
    result.positions[child.id] = { x: child.x ?? 0, y: child.y ?? 0 };
  }
  return result;
}
