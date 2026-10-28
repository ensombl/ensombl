import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { EdgeKind } from "@/shared/bindings/EdgeKind"
import type { SymbolNode } from "@/shared/bindings/SymbolNode"

/**
 * Pure projection of the architecture model into what the diagram shows:
 * package → nested folders → symbol. Collapsed groups hide their contents and
 * edges between hidden symbols are lifted to the nearest visible ancestors.
 */

export interface ViewFilters {
  hideTests: boolean
  /** Free functions and type aliases; off by default to keep the map about structure. */
  showFunctions: boolean
  edgeKinds: EdgeKind[]
  /** Show every group-level (aggregated) edge, not just the selected node's. */
  allGroupEdges: boolean
}

export interface ViewState {
  /** Expanded package/folder ids. */
  expanded: ReadonlySet<string>
  filters: ViewFilters
  selected?: string | null
}

export type ViewNode =
  | {
      type: "group"
      id: string
      level: "package" | "folder"
      label: string
      /** Full repo-relative path (folders) or package name. */
      title: string
      /** Package the group belongs to, for color coding. */
      pkg: string
      parent?: string
      expanded: boolean
      symbolCount: number
    }
  | { type: "symbol"; id: string; parent?: string; pkg: string; symbol: SymbolNode }

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
const STRUCTURAL = new Set<SymbolNode["kind"]>(["class", "interface", "enum"])

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

/** A folder group after compressing single-child chains (`src/modules` is one group). */
export interface FolderGroup {
  id: string
  label: string
  /** Repo-relative directory. */
  path: string
  parent: string
  children: string[]
  symbols: SymbolNode[]
  /** Symbols in this folder and below. */
  total: number
}

export interface PackageGroup {
  name: string
  /** Top-level folders under the package root. */
  children: string[]
  /** Symbols sitting directly in the package root. */
  symbols: SymbolNode[]
  total: number
}

export interface Hierarchy {
  packages: Map<string, PackageGroup>
  folders: Map<string, FolderGroup>
  /** Symbol id to its ancestor group ids, outermost first. */
  ancestors: Map<string, string[]>
}

interface TrieNode {
  path: string
  children: Map<string, TrieNode>
  symbols: SymbolNode[]
}

/** Follows a chain of symbol-less, single-child folders down to where it branches. */
function compress(seg: string, node: TrieNode): { label: string; node: TrieNode } {
  let label = seg
  let current = node
  while (current.symbols.length === 0 && current.children.size === 1) {
    const [[nextSeg, next]] = [...current.children]
    label = `${label}/${nextSeg}`
    current = next
  }
  return { label, node: current }
}

const sortedChildren = (node: TrieNode) => [...node.children].sort(([a], [b]) => a.localeCompare(b))

// Building the hierarchy walks every symbol; cache it per symbol array.
const hierarchyCache = new WeakMap<SymbolNode[], Hierarchy>()

export function buildHierarchy(symbols: SymbolNode[]): Hierarchy {
  let cached = hierarchyCache.get(symbols)
  if (!cached) {
    cached = computeHierarchy(symbols)
    hierarchyCache.set(symbols, cached)
  }
  return cached
}

function computeHierarchy(symbols: SymbolNode[]): Hierarchy {
  const byPackage = new Map<string, SymbolNode[]>()
  for (const s of symbols) {
    const list = byPackage.get(s.package) ?? []
    list.push(s)
    byPackage.set(s.package, list)
  }

  const packages = new Map<string, PackageGroup>()
  const folders = new Map<string, FolderGroup>()
  const ancestors = new Map<string, string[]>()

  for (const [pkg, list] of byPackage) {
    const prefix = commonDirPrefix([...new Set(list.map((s) => dirname(s.file)))])
    const root: TrieNode = { path: prefix, children: new Map(), symbols: [] }
    for (const s of list) {
      const rel = dirname(s.file).slice(prefix.length).replace(/^\//, "")
      let node = root
      for (const seg of rel ? rel.split("/") : []) {
        let child = node.children.get(seg)
        if (!child) {
          child = { path: node.path ? `${node.path}/${seg}` : seg, children: new Map(), symbols: [] }
          node.children.set(seg, child)
        }
        node = child
      }
      node.symbols.push(s)
    }

    const pid = packageId(pkg)
    const group: PackageGroup = { name: pkg, children: [], symbols: root.symbols, total: list.length }
    packages.set(pkg, group)
    for (const s of root.symbols) ancestors.set(s.id, [pid])

    const visit = (seg: string, raw: TrieNode, parent: string, chain: string[]): string => {
      const { label, node } = compress(seg, raw)
      const id = folderId(pkg, node.path)
      const folder: FolderGroup = { id, label, path: node.path, parent, children: [], symbols: node.symbols, total: 0 }
      folders.set(id, folder)
      const path = [...chain, id]
      for (const s of node.symbols) ancestors.set(s.id, path)
      folder.total = node.symbols.length
      for (const [childSeg, child] of sortedChildren(node)) {
        const childId = visit(childSeg, child, id, path)
        folder.children.push(childId)
        folder.total += folders.get(childId)?.total ?? 0
      }
      return id
    }

    for (const [seg, child] of sortedChildren(root)) group.children.push(visit(seg, child, pid, [pid]))
  }
  return { packages, folders, ancestors }
}

const filterCache = new WeakMap<SymbolNode[], Map<string, SymbolNode[]>>()

/** Symbols passing the filters; the same array is returned for the same inputs so caches hit. */
export function filterSymbols(model: ArchModel, filters: ViewFilters): SymbolNode[] {
  const key = `${filters.hideTests}:${filters.showFunctions}`
  let byKey = filterCache.get(model.symbols)
  if (!byKey) {
    byKey = new Map()
    filterCache.set(model.symbols, byKey)
  }
  let result = byKey.get(key)
  if (!result) {
    result = model.symbols.filter(
      (s) => (!filters.hideTests || !TEST_FILE.test(s.file)) && (filters.showFunctions || STRUCTURAL.has(s.kind)),
    )
    byKey.set(key, result)
  }
  return result
}

/** Small models start fully expanded; big ones start at package level. */
export function defaultExpanded(model: ArchModel, filters: ViewFilters, threshold = 120): Set<string> {
  const symbols = filterSymbols(model, filters)
  if (symbols.length > threshold) return new Set()
  const { packages, folders } = buildHierarchy(symbols)
  return new Set([...[...packages.keys()].map(packageId), ...folders.keys()])
}

/**
 * Expands one more level of whatever is currently visible. Keeps huge repos
 * from exploding into thousands of nodes in a single click.
 */
export function expandOneLevel(model: ArchModel, filters: ViewFilters, expanded: ReadonlySet<string>): Set<string> {
  const { packages, folders } = buildHierarchy(filterSymbols(model, filters))
  const next = new Set(expanded)
  const walk = (id: string, children: string[]) => {
    if (!expanded.has(id)) {
      next.add(id)
      return
    }
    for (const child of children) walk(child, folders.get(child)?.children ?? [])
  }
  for (const [pkg, group] of packages) walk(packageId(pkg), group.children)
  return next
}

export function buildView(model: ArchModel, state: ViewState): DiagramView {
  const symbols = filterSymbols(model, state.filters)
  const { packages, folders } = buildHierarchy(symbols)
  const nodes: ViewNode[] = []
  const visibleOf = new Map<string, string>()

  const hide = (groupId: string, folderIds: string[], direct: SymbolNode[]) => {
    for (const s of direct) visibleOf.set(s.id, groupId)
    for (const fid of folderIds) {
      const f = folders.get(fid)
      if (f) hide(groupId, f.children, f.symbols)
    }
  }

  const emitSymbols = (pkg: string, parent: string, list: SymbolNode[]) => {
    for (const s of list) {
      nodes.push({ type: "symbol", id: s.id, parent, pkg, symbol: s })
      visibleOf.set(s.id, s.id)
    }
  }

  const emitFolder = (pkg: string, fid: string) => {
    const f = folders.get(fid)
    if (!f) return
    const expanded = state.expanded.has(fid)
    nodes.push({
      type: "group",
      id: fid,
      level: "folder",
      label: f.label,
      title: f.path,
      pkg,
      parent: f.parent,
      expanded,
      symbolCount: f.total,
    })
    if (!expanded) return hide(fid, f.children, f.symbols)
    for (const child of f.children) emitFolder(pkg, child)
    emitSymbols(pkg, fid, f.symbols)
  }

  for (const [pkg, group] of [...packages].sort(([a], [b]) => a.localeCompare(b))) {
    const pid = packageId(pkg)
    const expanded = state.expanded.has(pid)
    nodes.push({ type: "group", id: pid, level: "package", label: pkg, title: pkg, pkg, expanded, symbolCount: group.total })
    if (!expanded) {
      hide(pid, group.children, group.symbols)
      continue
    }
    for (const fid of group.children) emitFolder(pkg, fid)
    emitSymbols(pkg, pid, group.symbols)
  }

  const kinds = new Set(state.filters.edgeKinds)
  const merged = new Map<string, ViewEdge>()
  for (const e of model.edges) {
    if (!kinds.has(e.kind)) continue
    const source = visibleOf.get(e.from)
    const target = visibleOf.get(e.to)
    if (!source || !target || source === target) continue

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

  const selected = state.selected ?? null
  const edges = [...merged.values()].filter((e) => {
    if (e.kind !== "aggregate") return strongest.get(`${e.source}->${e.target}`) === e
    // Group-level edges are noise at scale: only the selected node's, unless asked for all.
    return state.filters.allGroupEdges || e.source === selected || e.target === selected
  })

  return { nodes, edges }
}

/** Group ids that must be expanded for `symbolId` to be visible. */
export function ancestorsOf(model: ArchModel, symbolId: string): string[] {
  return buildHierarchy(model.symbols).ancestors.get(symbolId) ?? []
}
