import type { ElkNode } from "elkjs/lib/elk-api"

import type { Layer } from "@/shared/bindings/Layer"

import type { ComponentsView, ComponentsViewNode } from "../model/system-view"
import { flattenLayout, type Layout } from "./elk-graph"

export const SYSTEM_SIZES = {
  component: { width: 220, height: 60 },
  feature: { width: 230, height: 76 },
}

/** Left-to-right order of lanes; channels sit between application and domain. */
const LANE_ORDER: Record<Layer | "channel", number> = {
  interface: 0,
  application: 2,
  channel: 3,
  domain: 4,
  data: 6,
  external: 8,
}

export const LAYER_LABEL: Record<Layer, string> = {
  interface: "Interface",
  application: "Application",
  domain: "Domain",
  data: "Data",
  external: "External",
}

const lane = (n: ComponentsViewNode): number => {
  if (n.type === "feature") return 0
  if (n.node.kind === "channel") return LANE_ORDER.channel
  return LANE_ORDER[n.node.layer]
}

export const sizeOf = (n: ComponentsViewNode) => (n.type === "feature" ? SYSTEM_SIZES.feature : SYSTEM_SIZES.component)

/** Flat layered graph, flowing right: callers on the left, what they depend on to the right. */
export function toSystemElk(view: ComponentsView): ElkNode {
  return {
    id: "root",
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": "RIGHT",
      "elk.edgeRouting": "ORTHOGONAL",
      "elk.partitioning.activate": "true",
      "elk.separateConnectedComponents": "false",
      "elk.spacing.nodeNode": "22",
      "elk.layered.spacing.nodeNodeBetweenLayers": "90",
      "elk.spacing.edgeNode": "16",
      "elk.layered.spacing.edgeEdgeBetweenLayers": "10",
      "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
      "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
    },
    children: view.nodes.map((n) => ({
      id: n.id,
      ...sizeOf(n),
      layoutOptions: { "elk.partitioning.partition": String(lane(n)) },
    })),
    edges: view.edges.map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] })),
  }
}

export { flattenLayout, type Layout }

export interface Lane {
  layer: Layer
  x: number
  y: number
  width: number
  height: number
}

/** Background bands behind each layer's column, from the laid-out boxes. */
export function lanesOf(view: ComponentsView, layout: Layout): Lane[] {
  if (view.overview) return []
  const bounds = new Map<Layer, { x0: number; x1: number }>()
  let y0 = Infinity
  let y1 = -Infinity
  for (const n of view.nodes) {
    const box = layout.nodes.get(n.id)
    if (!box || n.type !== "component" || n.node.kind === "channel") continue
    const b = bounds.get(n.node.layer) ?? { x0: Infinity, x1: -Infinity }
    b.x0 = Math.min(b.x0, box.x)
    b.x1 = Math.max(b.x1, box.x + box.width)
    bounds.set(n.node.layer, b)
    y0 = Math.min(y0, box.y)
    y1 = Math.max(y1, box.y + box.height)
  }
  const pad = 16
  return [...bounds].map(([layer, b]) => ({
    layer,
    x: b.x0 - pad,
    y: y0 - pad - 28,
    width: b.x1 - b.x0 + pad * 2,
    height: y1 - y0 + pad * 2 + 28,
  }))
}

export function systemLayoutSignature(view: ComponentsView): string {
  return `${view.nodes.map((n) => n.id).join(";")}#${view.edges.map((e) => e.id).join(";")}`
}
