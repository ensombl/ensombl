import type { ElkExtendedEdge, ElkNode } from "elkjs/lib/elk-api"

import type { DiagramView, ViewNode } from "../model/view"
import { nodeSize, SIZES } from "./sizes"

const ROOT_OPTIONS: Record<string, string> = {
  "elk.algorithm": "layered",
  // Dependencies point up: what you depend on sits above you, like UML parents.
  "elk.direction": "UP",
  "elk.hierarchyHandling": "INCLUDE_CHILDREN",
  "elk.layered.spacing.nodeNodeBetweenLayers": "64",
  "elk.spacing.nodeNode": "36",
  "elk.layered.nodePlacement.strategy": "BRANDES_KOEPF",
  "elk.layered.crossingMinimization.strategy": "LAYER_SWEEP",
  "elk.edgeRouting": "ORTHOGONAL",
  "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
}

const GROUP_OPTIONS: Record<string, string> = {
  "elk.padding": `[top=${SIZES.groupPadding.top},left=${SIZES.groupPadding.side},bottom=${SIZES.groupPadding.bottom},right=${SIZES.groupPadding.side}]`,
}

const isContainer = (n: ViewNode) => n.type === "group" && n.expanded

/** Converts the view into a nested ELK graph; expanded groups become compound nodes. */
export function toElkGraph(view: DiagramView): ElkNode {
  const children = new Map<string | undefined, ViewNode[]>()
  for (const node of view.nodes) {
    const list = children.get(node.parent) ?? []
    list.push(node)
    children.set(node.parent, list)
  }

  const build = (node: ViewNode): ElkNode => {
    if (isContainer(node)) {
      return { id: node.id, layoutOptions: GROUP_OPTIONS, children: (children.get(node.id) ?? []).map(build) }
    }
    return { id: node.id, ...nodeSize(node) }
  }

  const edges: ElkExtendedEdge[] = view.edges.map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] }))
  return { id: "root", layoutOptions: ROOT_OPTIONS, children: (children.get(undefined) ?? []).map(build), edges }
}

export interface LaidOutNode {
  id: string
  x: number
  y: number
  width: number
  height: number
}

/** Flattens ELK output to positions relative to each node's parent. */
export function flattenLayout(root: ElkNode): Map<string, LaidOutNode> {
  const out = new Map<string, LaidOutNode>()
  const walk = (node: ElkNode) => {
    for (const child of node.children ?? []) {
      out.set(child.id, { id: child.id, x: child.x ?? 0, y: child.y ?? 0, width: child.width ?? 0, height: child.height ?? 0 })
      walk(child)
    }
  }
  walk(root)
  return out
}
