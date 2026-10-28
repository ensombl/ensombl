import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { EdgeKind } from "@/shared/bindings/EdgeKind"
import type { SymbolNode } from "@/shared/bindings/SymbolNode"

/**
 * Pure projection of the architecture model into what the diagram shows:
 * package → folder → symbol, where collapsed groups hide their contents and
 * edges between hidden symbols are lifted to the nearest visible ancestors.
 */

export interface ViewFilters {
  hideTests: boolean
  edgeKinds: EdgeKind[]
}

export interface ViewState {
  /** Expanded package/folder ids. */
  expanded: ReadonlySet<string>
  /** Symbols showing their members. */
  open: ReadonlySet<string>
  filters: ViewFilters
}

export type ViewNode =
  | {
      type: "group"
      id: string
      level: "package" | "folder"
      label: string
      parent?: string
      expanded: boolean
      symbolCount: number
    }
  | { type: "symbol"; id: string; parent?: string; symbol: SymbolNode; open: boolean }

export interface ViewEdge {
  id: string
  source: string
  target: string
  /** `aggregate` when either end is a collapsed group. */
  kind: EdgeKind | "aggregate"
  count: number
}

export interface DiagramView {
  nodes: ViewNode[]
  edges: ViewEdge[]
}

export const ALL_EDGE_KINDS: EdgeKind[] = ["extends", "implements", "association", "dependency"]
const KIND_STRENGTH: EdgeKind[] = ["extends", "implements", "association", "dependency"]

const TEST_FILE = /(^|\/)(__tests__|__mocks__|test|tests|e2e)\/|\.(test|spec|stories)\.[cm]?[jt]sx?$/

export const packageId = (name: string) => `pkg:${name}`
export const folderId = (pkg: string, dir: string) => `dir:${pkg}:${dir}`

const dirname = (file: string) => {
  const i = file.lastIndexOf("/")
  return i < 0 ? "" : file.slice(0, i)
}

function commonDirPrefix(dirs: string[]): string {
  if (dirs.length === 0) return ""
  let prefix = dirs[0].split("/")
  for (const dir of dirs.slice(1)) {
    const parts = dir.split("/")
    let i = 0
    while (i < prefix.length && i < parts.length && prefix[i] === parts[i]) i++
    prefix = prefix.slice(0, i)
  }
  return prefix.join("/")
}

/** Where each symbol sits in the package → folder hierarchy. */
export interface Hierarchy {
  packages: Map<string, { folders: Map<string, string>; symbols: SymbolNode[] }>
  /** Symbol id to its ancestor group ids, outermost first. */
  ancestors: Map<string, string[]>
}

export function buildHierarchy(symbols: SymbolNode[]): Hierarchy {
  const byPackage = new Map<string, SymbolNode[]>()
  for (const s of symbols) {
    const list = byPackage.get(s.package) ?? []
    list.push(s)
    byPackage.set(s.package, list)
  }

  const packages: Hierarchy["packages"] = new Map()
  const ancestors = new Map<string, string[]>()
  for (const [pkg, list] of byPackage) {
    const dirs = [...new Set(list.map((s) => dirname(s.file)))]
    const prefix = commonDirPrefix(dirs)
    const folders = new Map<string, string>()
    // A package with a single folder doesn't need a folder level.
    const useFolders = dirs.length > 1
    for (const s of list) {
      const chain = [packageId(pkg)]
      if (useFolders) {
        const dir = dirname(s.file)
        const label = dir.slice(prefix.length).replace(/^\//, "") || "."
        const id = folderId(pkg, dir)
        folders.set(id, label)
        chain.push(id)
      }
      ancestors.set(s.id, chain)
    }
    packages.set(pkg, { folders, symbols: list })
  }
  return { packages, ancestors }
}

export function filterSymbols(model: ArchModel, filters: ViewFilters): SymbolNode[] {
  return filters.hideTests ? model.symbols.filter((s) => !TEST_FILE.test(s.file)) : model.symbols
}

/** Small models start fully expanded; big ones start at package level. */
export function defaultExpanded(model: ArchModel, threshold = 120): Set<string> {
  if (model.symbols.length > threshold) return new Set()
  const { packages } = buildHierarchy(model.symbols)
  const ids = new Set<string>()
  for (const [pkg, { folders }] of packages) {
    ids.add(packageId(pkg))
    for (const id of folders.keys()) ids.add(id)
  }
  return ids
}

/**
 * Expands one more level: all packages first, then all folders. Keeps huge
 * repos from exploding into thousands of nodes in a single click.
 */
export function expandOneLevel(model: ArchModel, expanded: ReadonlySet<string>): Set<string> {
  const { packages } = buildHierarchy(model.symbols)
  const packageIds = [...packages.keys()].map(packageId)
  const next = new Set(expanded)
  if (packageIds.some((id) => !expanded.has(id))) {
    for (const id of packageIds) next.add(id)
    return next
  }
  for (const { folders } of packages.values()) for (const id of folders.keys()) next.add(id)
  return next
}

export function buildView(model: ArchModel, state: ViewState): DiagramView {
  const symbols = filterSymbols(model, state.filters)
  const { packages, ancestors } = buildHierarchy(symbols)
  const nodes: ViewNode[] = []
  const visibleOf = new Map<string, string>()

  for (const [pkg, { folders, symbols: list }] of [...packages].sort(([a], [b]) => a.localeCompare(b))) {
    const pid = packageId(pkg)
    const pkgOpen = state.expanded.has(pid)
    nodes.push({ type: "group", id: pid, level: "package", label: pkg, expanded: pkgOpen, symbolCount: list.length })
    if (!pkgOpen) {
      for (const s of list) visibleOf.set(s.id, pid)
      continue
    }

    const folderCounts = new Map<string, number>()
    for (const s of list) {
      const folder = ancestors.get(s.id)?.[1]
      if (folder) folderCounts.set(folder, (folderCounts.get(folder) ?? 0) + 1)
    }
    for (const [fid, label] of [...folders].sort(([, a], [, b]) => a.localeCompare(b))) {
      nodes.push({
        type: "group",
        id: fid,
        level: "folder",
        label,
        parent: pid,
        expanded: state.expanded.has(fid),
        symbolCount: folderCounts.get(fid) ?? 0,
      })
    }

    for (const s of list) {
      const folder = ancestors.get(s.id)?.[1]
      if (folder && !state.expanded.has(folder)) {
        visibleOf.set(s.id, folder)
        continue
      }
      nodes.push({ type: "symbol", id: s.id, parent: folder ?? pid, symbol: s, open: state.open.has(s.id) })
      visibleOf.set(s.id, s.id)
    }
  }

  const kinds = new Set(state.filters.edgeKinds)
  const symbolIds = new Set(symbols.map((s) => s.id))
  const merged = new Map<string, ViewEdge>()
  for (const e of model.edges) {
    if (!kinds.has(e.kind)) continue
    const source = visibleOf.get(e.from)
    const target = visibleOf.get(e.to)
    if (!source || !target || source === target) continue
    if (!symbolIds.has(e.from) || !symbolIds.has(e.to)) continue

    const lifted = source !== e.from || target !== e.to
    const kind: ViewEdge["kind"] = lifted ? "aggregate" : e.kind
    const key = lifted ? `${source}->${target}` : `${source}->${target}:${e.kind}`
    const existing = merged.get(key)
    if (existing) existing.count++
    else merged.set(key, { id: key, source, target, kind, count: 1 })
  }

  // Between the same two symbols, only the strongest relation is drawn.
  const strongest = new Map<string, ViewEdge>()
  for (const edge of merged.values()) {
    if (edge.kind === "aggregate") continue
    const pair = `${edge.source}->${edge.target}`
    const current = strongest.get(pair)
    if (!current || KIND_STRENGTH.indexOf(edge.kind) < KIND_STRENGTH.indexOf(current.kind as EdgeKind)) {
      strongest.set(pair, edge)
    }
  }
  const edges = [...merged.values()].filter((e) => e.kind === "aggregate" || strongest.get(`${e.source}->${e.target}`) === e)

  return { nodes, edges }
}

/** Group ids that must be expanded for `symbolId` to be visible. */
export function ancestorsOf(model: ArchModel, symbolId: string): string[] {
  return buildHierarchy(model.symbols).ancestors.get(symbolId) ?? []
}
