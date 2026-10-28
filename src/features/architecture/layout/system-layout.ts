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
  if (n.type === "feature" || n.type === "package") return 0
  if (n.node.kind === "channel") return LANE_ORDER.channel
  return LANE_ORDER[n.node.layer]
}

const MAX_WIDTH = 360

/** Wide enough for the name (icon, padding and badges included), so labels aren't cut off. */
export function sizeOf(n: ComponentsViewNode): { width: number; height: number } {
  if (n.type === "package") return { width: 0, height: 0 }
  if (n.type === "feature") {
    const width = Math.min(Math.max(SYSTEM_SIZES.feature.width, n.label.length * 8.6 + 64), MAX_WIDTH)
    return { width: Math.ceil(width), height: SYSTEM_SIZES.feature.height }
  }
  const badge = n.entryCount > 0 ? 36 : 0
  const width = Math.min(Math.max(SYSTEM_SIZES.component.width, n.node.name.length * 7.2 + 56 + badge), MAX_WIDTH)
  return { width: Math.ceil(width), height: SYSTEM_SIZES.component.height }
}

const BASE_OPTIONS: Record<string, string> = {
  "elk.algorithm": "layered",
  "elk.direction": "RIGHT",
  "elk.edgeRouting": "ORTHOGONAL",
  "elk.spacing.nodeNode": "22",
  "elk.layered.spacing.nodeNodeBetweenLayers": "90",
  "elk.spacing.edgeNode": "16",
  "elk.layered.spacing.edgeEdgeBetweenLayers": "10",
  "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
  "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
  // Many callers of one component share a trunk instead of fanning in.
  "elk.layered.mergeEdges": "true",
}

/**
 * Layered graph flowing right: callers on the left, what they depend on to
 * the right. Ungrouped views are split into layer lanes; grouped ones nest
 * each package in a box and route edges across boxes.
 */
export function toSystemElk(view: ComponentsView): ElkNode {
  const leaf = (n: ComponentsViewNode): ElkNode => ({
    id: n.id,
    ...sizeOf(n),
    layoutOptions: view.grouped ? undefined : { "elk.partitioning.partition": String(lane(n)) },
  })
  const edges = view.edges.map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] }))
  if (!view.grouped) {
    return {
      id: "root",
      layoutOptions: { ...BASE_OPTIONS, "elk.partitioning.activate": "true", "elk.separateConnectedComponents": "false" },
      children: view.nodes.map(leaf),
      edges,
    }
  }
  const childrenOf = new Map<string, ComponentsViewNode[]>()
  for (const n of view.nodes) {
    if (n.type === "package" || !n.parent) continue
    childrenOf.set(n.parent, [...(childrenOf.get(n.parent) ?? []), n])
  }
  return {
    id: "root",
    layoutOptions: {
      ...BASE_OPTIONS,
      "elk.hierarchyHandling": "INCLUDE_CHILDREN",
      // Edges cross boxes, so give their routes in root coordinates.
      "elk.json.edgeCoords": "ROOT",
      "elk.spacing.componentComponent": "60",
    },
    children: view.nodes
      .filter((n) => n.type === "package" || !n.parent)
      .map((n) =>
        n.type === "package"
          ? {
              id: n.id,
              layoutOptions: { "elk.padding": `[top=${PACKAGE_HEADER + 16},left=20,bottom=20,right=20]` },
              children: (childrenOf.get(n.id) ?? []).map(leaf),
            }
          : leaf(n),
      ),
    edges,
  }
}

/** Room for the package name at the top of its box. */
export const PACKAGE_HEADER = 28

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
  if (view.overview || view.grouped) return []
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
  const nodes = view.nodes.map((n) => `${n.id}|${sizeOf(n).width}`)
  return `${nodes.join(";")}#${view.edges.map((e) => e.id).join(";")}`
}
