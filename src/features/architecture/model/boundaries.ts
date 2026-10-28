import type { Layer } from "@/shared/bindings/Layer"
import type { SystemModel } from "@/shared/bindings/SystemModel"
import type { SystemNode } from "@/shared/bindings/SystemNode"

import { featureId } from "./system-view"

export const LAYERS: Layer[] = ["interface", "application", "domain", "data"]

export interface FeatureRow {
  id: string
  label: string
  pkg: string
  byLayer: Record<Layer, number>
  total: number
}

const isComponent = (n: SystemNode) => n.kind === "component" && !n.hidden && n.role !== "other"

/** How many components each feature has per layer, biggest features first. */
export function featureLayers(model: SystemModel): FeatureRow[] {
  const rows = new Map<string, FeatureRow>()
  for (const n of model.nodes.filter(isComponent)) {
    const id = featureId(n)
    let row = rows.get(id)
    if (!row) {
      row = { id, label: n.feature, pkg: n.package, byLayer: { interface: 0, application: 0, domain: 0, data: 0, external: 0 }, total: 0 }
      rows.set(id, row)
    }
    row.byLayer[n.layer]++
    row.total++
  }
  return [...rows.values()].sort((a, b) => b.total - a.total || a.label.localeCompare(b.label))
}

export interface FeatureMatrix {
  features: { id: string; label: string }[]
  /** `cells[from][to]`: calls, injections and requests between the two. */
  cells: number[][]
  max: number
}

/** Who depends on whom between features, for the `limit` most connected ones. */
export function featureMatrix(model: SystemModel, limit = 24): FeatureMatrix {
  const featureOf = new Map<string, { id: string; label: string }>()
  for (const n of model.nodes.filter(isComponent)) featureOf.set(n.id, { id: featureId(n), label: n.feature })
  const counts = new Map<string, Map<string, number>>()
  const degree = new Map<string, number>()
  const labels = new Map<string, string>()
  for (const i of model.interactions) {
    if (i.kind !== "call" && i.kind !== "inject" && i.kind !== "http") continue
    const from = featureOf.get(i.from)
    const to = featureOf.get(i.to)
    if (!from || !to || from.id === to.id) continue
    labels.set(from.id, from.label)
    labels.set(to.id, to.label)
    const row = counts.get(from.id) ?? new Map()
    row.set(to.id, (row.get(to.id) ?? 0) + i.count)
    counts.set(from.id, row)
    degree.set(from.id, (degree.get(from.id) ?? 0) + i.count)
    degree.set(to.id, (degree.get(to.id) ?? 0) + i.count)
  }
  const features = [...degree.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .slice(0, limit)
    .map(([id]) => ({ id, label: labels.get(id) ?? id }))
  let max = 0
  const cells = features.map((from) =>
    features.map((to) => {
      const n = counts.get(from.id)?.get(to.id) ?? 0
      max = Math.max(max, n)
      return n
    }),
  )
  return { features, cells, max }
}
