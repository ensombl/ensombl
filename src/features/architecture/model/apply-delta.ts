import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { Edge } from "@/shared/bindings/Edge"
import type { ModelDelta } from "@/shared/bindings/ModelDelta"

const edgeKey = (e: Edge) => `${e.from}\u0000${e.to}\u0000${e.kind}`

/** Applies an incremental reindex result without refetching the whole model. */
export function applyDelta(model: ArchModel, delta: ModelDelta): ArchModel {
  const removed = new Set(delta.removed)
  const upserted = new Map(delta.upserted.map((s) => [s.id, s]))
  const symbols = model.symbols.filter((s) => !removed.has(s.id) && !upserted.has(s.id))
  symbols.push(...upserted.values())
  symbols.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))

  const dropped = new Set(delta.edgesRemoved.map(edgeKey))
  const edges = model.edges.filter((e) => !dropped.has(edgeKey(e)) && !removed.has(e.from) && !removed.has(e.to))
  edges.push(...delta.edgesAdded)
  return { ...model, symbols, edges }
}
