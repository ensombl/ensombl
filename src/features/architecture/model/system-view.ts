import type { Interaction } from "@/shared/bindings/Interaction"
import type { InteractionKind } from "@/shared/bindings/InteractionKind"
import type { Role } from "@/shared/bindings/Role"
import type { SystemModel } from "@/shared/bindings/SystemModel"
import type { SystemNode } from "@/shared/bindings/SystemNode"

/**
 * Pure projection of the system model for the Components tab. Big systems
 * open as an overview of features; focusing a feature shows its components
 * plus whatever they talk to, so the map stays local instead of a hairball.
 */

export interface SystemFilters {
  /** Classes the detector couldn't place. */
  showUnclassified: boolean
  showHidden: boolean
  showExternal: boolean
  showChannels: boolean
  kinds: InteractionKind[]
}

export const ALL_KINDS: InteractionKind[] = ["call", "http", "emits", "handles", "renders", "inject"]

export const defaultFilters = (): SystemFilters => ({
  showUnclassified: false,
  showHidden: false,
  showExternal: true,
  showChannels: true,
  // Injection mostly duplicates calls and doubles the lines.
  kinds: ALL_KINDS.filter((k) => k !== "inject"),
})

/** Shown as components without the overview step when this small. */
export const OVERVIEW_THRESHOLD = 60

export type ComponentsViewNode =
  | {
      type: "feature"
      id: string
      label: string
      pkg: string
      count: number
      entryCount: number
      roles: Partial<Record<Role, number>>
    }
  | {
      type: "component"
      id: string
      node: SystemNode
      entryCount: number
      /** A neighbor shown for context, outside the focused feature. */
      context: boolean
    }

export interface ComponentsViewEdge {
  id: string
  source: string
  target: string
  kind: InteractionKind | "aggregate"
  members: string[]
  count: number
  interaction?: Interaction
}

export interface ComponentsView {
  nodes: ComponentsViewNode[]
  edges: ComponentsViewEdge[]
  /** Overview of features rather than components. */
  overview: boolean
}

export const featureId = (n: Pick<SystemNode, "package" | "feature">) => `feature:${n.package}/${n.feature}`

export function visibleNodes(model: SystemModel, filters: SystemFilters): SystemNode[] {
  return model.nodes.filter(
    (n) =>
      (filters.showHidden || !n.hidden) &&
      (filters.showUnclassified || n.role !== "other") &&
      (filters.showExternal || n.kind !== "external") &&
      (filters.showChannels || n.kind !== "channel"),
  )
}

function entryCounts(model: SystemModel): Map<string, number> {
  const counts = new Map<string, number>()
  for (const e of model.entryPoints) counts.set(e.node, (counts.get(e.node) ?? 0) + 1)
  return counts
}

const pushMembers = (into: string[], from: string[]) => {
  for (const m of from) if (!into.includes(m)) into.push(m)
}

/** Features (and externals) with interactions summed between them; events pass through channels. */
function overview(model: SystemModel, nodes: SystemNode[], kinds: Set<InteractionKind>): ComponentsView {
  const entries = entryCounts(model)
  const groupOf = new Map<string, string>()
  const features = new Map<string, Extract<ComponentsViewNode, { type: "feature" }>>()
  const out: ComponentsViewNode[] = []
  for (const n of nodes) {
    if (n.kind === "channel") continue
    if (n.kind === "external") {
      groupOf.set(n.id, n.id)
      out.push({ type: "component", id: n.id, node: n, entryCount: 0, context: false })
      continue
    }
    const id = featureId(n)
    groupOf.set(n.id, id)
    let f = features.get(id)
    if (!f) {
      f = { type: "feature", id, label: n.feature, pkg: n.package, count: 0, entryCount: 0, roles: {} }
      features.set(id, f)
    }
    f.count++
    f.entryCount += entries.get(n.id) ?? 0
    f.roles[n.role] = (f.roles[n.role] ?? 0) + 1
  }
  out.unshift(...features.values())

  const edges = new Map<string, ComponentsViewEdge>()
  const add = (from: string | undefined, to: string | undefined, members: string[], count: number) => {
    if (!from || !to || from === to) return
    const id = `${from}->${to}`
    const edge = edges.get(id)
    if (edge) {
      edge.count += count
      pushMembers(edge.members, members)
    } else edges.set(id, { id, source: from, target: to, kind: "aggregate", members: [...members], count })
  }
  const handlers = new Map<string, Interaction[]>()
  for (const i of model.interactions) {
    if (i.kind === "handles") handlers.set(i.from, [...(handlers.get(i.from) ?? []), i])
  }
  for (const i of model.interactions) {
    if (!kinds.has(i.kind) || i.kind === "handles") continue
    if (i.kind === "emits") {
      for (const h of handlers.get(i.to) ?? []) add(groupOf.get(i.from), groupOf.get(h.to), i.members, i.count)
      continue
    }
    add(groupOf.get(i.from), groupOf.get(i.to), i.members, i.count)
  }
  return { nodes: out, edges: [...edges.values()], overview: true }
}

/**
 * `focus`: a feature id, `"all"`, or `null` for the default (overview when
 * the system is big, everything when it's small).
 */
export function buildComponentsView(model: SystemModel, focus: string | null, filters: SystemFilters): ComponentsView {
  const nodes = visibleNodes(model, filters)
  const kinds = new Set(filters.kinds)
  const components = nodes.filter((n) => n.kind === "component")
  if (focus === null && components.length > OVERVIEW_THRESHOLD) return overview(model, nodes, kinds)

  const visible = new Map(nodes.map((n) => [n.id, n]))
  const interactions = model.interactions.filter((i) => kinds.has(i.kind) && visible.has(i.from) && visible.has(i.to))
  const entries = entryCounts(model)

  let core: Set<string>
  const context = new Set<string>()
  if (focus === null || focus === "all") {
    core = new Set(visible.keys())
  } else {
    core = new Set(components.filter((n) => featureId(n) === focus).map((n) => n.id))
    for (const i of interactions) {
      if (core.has(i.from) && !core.has(i.to)) context.add(i.to)
      if (core.has(i.to) && !core.has(i.from)) context.add(i.from)
    }
    // Follow channels through to who handles or emits on them.
    for (const i of interactions) {
      const channel = visible.get(i.kind === "handles" ? i.from : i.to)
      if (channel?.kind !== "channel" || !context.has(channel.id)) continue
      if (i.kind === "handles" && !core.has(i.to)) context.add(i.to)
      if (i.kind === "emits" && !core.has(i.from)) context.add(i.from)
    }
  }

  const included = new Set([...core, ...context])
  const out: ComponentsViewNode[] = [...included].map((id) => {
    const node = visible.get(id)!
    return { type: "component", id, node, entryCount: entries.get(id) ?? 0, context: context.has(id) && node.kind === "component" }
  })
  const edges: ComponentsViewEdge[] = interactions
    .filter((i) => included.has(i.from) && included.has(i.to) && (core.has(i.from) || core.has(i.to) || visible.get(i.from)?.kind === "channel" || visible.get(i.to)?.kind === "channel"))
    .map((i) => ({ id: i.id, source: i.from, target: i.to, kind: i.kind, members: i.members, count: i.count, interaction: i }))
  return { nodes: out, edges, overview: false }
}

/** Label for an edge: the first members, then how many more. */
export function membersLabel(members: string[], max = 2): string {
  if (members.length === 0) return ""
  const shown = members.slice(0, max).join(", ")
  return members.length > max ? `${shown} +${members.length - max}` : shown
}
