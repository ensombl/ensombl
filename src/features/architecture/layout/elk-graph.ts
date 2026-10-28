import type { ElkExtendedEdge, ElkNode } from "elkjs/lib/elk-api"

import type { DiagramView, ViewNode } from "../model/view"
import { nodeSize, SIZES } from "./sizes"

/**
 * Each container is laid out on its own (ELK's default hierarchy handling):
 * edges between siblings are routed orthogonally by ELK, and unconnected
 * nodes are packed into a balanced rectangle instead of one endless row.
 * Edges that cross container boundaries are drawn by the canvas.
 */
const LAYOUT_OPTIONS: Record<string, string> = {
  "elk.algorithm": "layered",
  // Dependencies point up: what you depend on sits above you, like UML parents.
  "elk.direction": "UP",
  "elk.edgeRouting": "ORTHOGONAL",
  "elk.aspectRatio": "1.6",
  "elk.separateConnectedComponents": "true",
  "elk.spacing.componentComponent": "28",
  "elk.layered.compaction.connectedComponents": "true",
  "elk.spacing.nodeNode": "24",
  "elk.layered.spacing.nodeNodeBetweenLayers": "44",
  "elk.spacing.edgeNode": "14",
  "elk.layered.spacing.edgeNodeBetweenLayers": "14",
  "elk.layered.spacing.edgeEdgeBetweenLayers": "8",
  "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
  "elk.layered.mergeEdges": "true",
}

const GROUP_OPTIONS: Record<string, string> = {
  ...LAYOUT_OPTIONS,
  "elk.padding": `[top=${SIZES.groupPadding.top},left=${SIZES.groupPadding.side},bottom=${SIZES.groupPadding.bottom},right=${SIZES.groupPadding.side}]`,
}

const isContainer = (n: ViewNode) => n.type === "group" && n.expanded

export interface ElkBuild {
  graph: ElkNode
  /** Edge ids ELK routes (both ends share a parent). Others are drawn freely. */
  routed: Set<string>
}

/** Converts the view into a nested ELK graph; expanded groups become compound nodes. */
export function toElkGraph(view: DiagramView): ElkBuild {
  const children = new Map<string | undefined, ViewNode[]>()
  const parentOf = new Map<string, string | undefined>()
  for (const node of view.nodes) {
    const list = children.get(node.parent) ?? []
    list.push(node)
    children.set(node.parent, list)
    parentOf.set(node.id, node.parent)
  }

  const edgesIn = new Map<string | undefined, ElkExtendedEdge[]>()
  const routed = new Set<string>()
  for (const e of view.edges) {
    const parent = parentOf.get(e.source)
    if (parent !== parentOf.get(e.target)) continue
    const list = edgesIn.get(parent) ?? []
    list.push({ id: e.id, sources: [e.source], targets: [e.target] })
    edgesIn.set(parent, list)
    routed.add(e.id)
  }

  const build = (node: ViewNode): ElkNode => {
    if (isContainer(node)) {
      return {
        id: node.id,
        layoutOptions: GROUP_OPTIONS,
        children: (children.get(node.id) ?? []).map(build),
        edges: edgesIn.get(node.id) ?? [],
      }
    }
    return { id: node.id, ...nodeSize(node) }
  }

  return {
    graph: {
      id: "root",
      layoutOptions: LAYOUT_OPTIONS,
      children: (children.get(undefined) ?? []).map(build),
      edges: edgesIn.get(undefined) ?? [],
    },
    routed,
  }
}

export interface LaidOutNode {
  id: string
  /** Relative to the parent node. */
  x: number
  y: number
  width: number
  height: number
}

export interface Point {
  x: number
  y: number
}

export interface Layout {
  nodes: Map<string, LaidOutNode>
  /** Routed edge polylines in absolute flow coordinates. */
  routes: Map<string, Point[]>
}

/** Flattens ELK output: node boxes relative to their parent, edge routes made absolute. */
export function flattenLayout(root: ElkNode): Layout {
  const nodes = new Map<string, LaidOutNode>()
  const routes = new Map<string, Point[]>()
  const walk = (node: ElkNode, ox: number, oy: number) => {
    for (const edge of node.edges ?? []) {
      const section = edge.sections?.[0]
      if (!section) continue
      const points = [section.startPoint, ...(section.bendPoints ?? []), section.endPoint]
      routes.set(edge.id, points.map((p) => ({ x: p.x + ox, y: p.y + oy })))
    }
    for (const child of node.children ?? []) {
      const x = child.x ?? 0
      const y = child.y ?? 0
      nodes.set(child.id, { id: child.id, x, y, width: child.width ?? 0, height: child.height ?? 0 })
      walk(child, ox + x, oy + y)
    }
  }
  walk(root, 0, 0)
  return { nodes, routes }
}
