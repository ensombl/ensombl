import { create } from "zustand"

import type { EdgeKind } from "@/shared/bindings/EdgeKind"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { ALL_EDGE_KINDS, type ViewFilters } from "./view"

/** Diagram UI state for one worktree tab. Client-only. */
export interface DiagramState {
  initialized: boolean
  expanded: Set<string>
  filters: ViewFilters
  /** Symbol shown in the inspector. */
  selected: string | null
  /** Nodes to center on once they're laid out. */
  focus: string[] | null
  /** Spotlighted pattern: participant symbol id to its role. Everything else dims. */
  spotlight: { patternId: string; roles: Record<string, string> } | null
  patternsOpen: boolean
}

const initial = (): DiagramState => ({
  initialized: false,
  expanded: new Set(),
  filters: { hideTests: true, showFunctions: false, edgeKinds: ALL_EDGE_KINDS, allGroupEdges: false },
  selected: null,
  focus: null,
  spotlight: null,
  patternsOpen: false,
})

interface DiagramStore {
  byWorktree: Record<string, DiagramState>
  patch: (worktreeId: string, fn: (s: DiagramState) => Partial<DiagramState>) => void
}

const useDiagramStore = create<DiagramStore>((set) => ({
  byWorktree: {},
  patch: (worktreeId, fn) =>
    set((store) => {
      const current = store.byWorktree[worktreeId] ?? initial()
      return { byWorktree: { ...store.byWorktree, [worktreeId]: { ...current, ...fn(current) } } }
    }),
}))

const toggle = (set: Set<string>, id: string) => {
  const next = new Set(set)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  return next
}

/** Diagram state and actions for the current worktree tab. */
export function useDiagram() {
  const { id } = useWorkspace()
  const state = useDiagramStore((s) => s.byWorktree[id]) ?? initial()
  const patch = useDiagramStore((s) => s.patch)
  const update = (fn: (s: DiagramState) => Partial<DiagramState>) => patch(id, fn)

  return {
    state,
    init: (expanded: Set<string>) => update((s) => (s.initialized ? {} : { initialized: true, expanded })),
    toggleGroup: (groupId: string) => update((s) => ({ expanded: toggle(s.expanded, groupId) })),
    setExpanded: (expanded: Set<string>) => update(() => ({ expanded })),
    collapseAll: () => update(() => ({ expanded: new Set() })),
    setFilters: (filters: Partial<ViewFilters>) => update((s) => ({ filters: { ...s.filters, ...filters } })),
    setEdgeKinds: (edgeKinds: EdgeKind[]) => update((s) => ({ filters: { ...s.filters, edgeKinds } })),
    select: (symbolId: string | null) => update(() => ({ selected: symbolId })),
    /** Expands the symbol's ancestors, selects it and centers on it. */
    reveal: (symbolId: string, ancestors: string[], needsFunctions = false) =>
      update((s) => ({
        expanded: new Set([...s.expanded, ...ancestors]),
        selected: symbolId,
        focus: [symbolId],
        filters: needsFunctions ? { ...s.filters, showFunctions: true } : s.filters,
      })),
    clearFocus: () => update(() => ({ focus: null })),
    /** Reveals all participants of a pattern and dims the rest of the map. */
    spotlight: (patternId: string, roles: Record<string, string>, ancestors: string[]) =>
      update((s) => ({
        expanded: new Set([...s.expanded, ...ancestors]),
        spotlight: { patternId, roles },
        focus: Object.keys(roles),
        selected: null,
      })),
    clearSpotlight: () => update(() => ({ spotlight: null })),
    togglePatterns: () => update((s) => ({ patternsOpen: !s.patternsOpen })),
  }
}
