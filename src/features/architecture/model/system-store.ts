import { create } from "zustand"

import { useWorkspace } from "@/shared/workspace/workspace-context"

import { defaultFilters, type SystemFilters } from "./system-view"

export type ArchitectureTab = "components" | "flows" | "boundaries" | "code"

/** Architecture-pane UI state for one worktree tab. Client-only. */
export interface SystemUiState {
  tab: ArchitectureTab
  /** Focused feature id, `"all"`, or `null` for the default view. */
  focus: string | null
  /** Component shown in the inspector. */
  selected: string | null
  filters: SystemFilters
  /** Entry point open in the Flows tab. */
  entry: string | null
}

const initial = (): SystemUiState => ({ tab: "components", focus: null, selected: null, filters: defaultFilters(), entry: null })

// One shared fallback so untouched tabs see a stable object (and stable memo deps).
const DEFAULT = initial()

interface Store {
  byWorktree: Record<string, SystemUiState>
  patch: (worktreeId: string, value: Partial<SystemUiState>) => void
}

const useStore = create<Store>((set) => ({
  byWorktree: {},
  patch: (id, value) => set((s) => ({ byWorktree: { ...s.byWorktree, [id]: { ...(s.byWorktree[id] ?? initial()), ...value } } })),
}))

export function useSystemUi() {
  const { id } = useWorkspace()
  const state = useStore((s) => s.byWorktree[id]) ?? DEFAULT
  const patch = useStore((s) => s.patch)
  const set = (value: Partial<SystemUiState>) => patch(id, value)
  return {
    state,
    setTab: (tab: ArchitectureTab) => set({ tab }),
    focus: (focus: string | null) => set({ focus, selected: null }),
    select: (selected: string | null) => set({ selected }),
    setFilters: (filters: Partial<SystemFilters>) => set({ filters: { ...state.filters, ...filters } }),
    openEntry: (entry: string | null) => set({ entry, tab: "flows" }),
  }
}
