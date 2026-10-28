import { create } from "zustand"

import { useWorkspace } from "@/shared/workspace/workspace-context"

import { closeTab, openTab } from "./open-files"
import { defaultFilters, type SystemFilters } from "./system-view"

export type ArchitectureTab = "components" | "flows" | "boundaries" | "classes" | "code"

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
  /** File open in the Code tab, and the line to show. */
  file: string | null
  line: number
  /** Files open as tabs in the Code tab. */
  openFiles: string[]
}

const initial = (): SystemUiState => ({ tab: "components", focus: null, selected: null, filters: defaultFilters(), entry: null, file: null, line: 1, openFiles: [] })

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
    openFile: (file: string, line = 1) => set({ file, line, openFiles: openTab(state.openFiles, state.file, file) }),
    closeFile: (file: string) => {
      const { tabs, active } = closeTab(state.openFiles, state.file, file)
      set({ openFiles: tabs, file: active, ...(active !== state.file && { line: 1 }) })
    },
    /** Jump to a file in the Code tab. */
    showFile: (file: string, line = 1) => set({ file, line, tab: "code", openFiles: openTab(state.openFiles, state.file, file) }),
  }
}
