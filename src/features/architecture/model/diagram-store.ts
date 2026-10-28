import { create } from "zustand"

import type { EdgeKind } from "@/shared/bindings/EdgeKind"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { ALL_EDGE_KINDS, type ViewFilters } from "./view"

/** Diagram UI state for one worktree tab. Client-only. */
export interface DiagramState {
  initialized: boolean
  expanded: Set<string>
  open: Set<string>
  filters: ViewFilters
  /** Symbol to center on once it's laid out. */
  focus: string | null
}

const initial = (): DiagramState => ({
  initialized: false,
  expanded: new Set(),
  open: new Set(),
  filters: { hideTests: true, edgeKinds: ALL_EDGE_KINDS },
  focus: null,
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
    toggleOpen: (symbolId: string) => update((s) => ({ open: toggle(s.open, symbolId) })),
    expandAll: (ids: string[]) => update(() => ({ expanded: new Set(ids) })),
    collapseAll: () => update(() => ({ expanded: new Set(), open: new Set() })),
    setHideTests: (hideTests: boolean) => update((s) => ({ filters: { ...s.filters, hideTests } })),
    setEdgeKinds: (edgeKinds: EdgeKind[]) => update((s) => ({ filters: { ...s.filters, edgeKinds } })),
    reveal: (symbolId: string, ancestors: string[]) =>
      update((s) => ({
        expanded: new Set([...s.expanded, ...ancestors]),
        open: new Set([...s.open, symbolId]),
        focus: symbolId,
      })),
    clearFocus: () => update(() => ({ focus: null })),
  }
}
