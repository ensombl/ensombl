import { create } from "zustand"

interface RemoveTarget {
  projectId: string
  path: string
  label: string
}

/** Which workspace dialog is open. Client-only UI state. */
interface WorkspaceDialogsState {
  createWorktreeFor: string | null
  removeWorktree: RemoveTarget | null
  openCreateWorktree: (projectId: string) => void
  openRemoveWorktree: (target: RemoveTarget) => void
  close: () => void
}

export const useWorkspaceDialogs = create<WorkspaceDialogsState>((set) => ({
  createWorktreeFor: null,
  removeWorktree: null,
  openCreateWorktree: (projectId) => set({ createWorktreeFor: projectId, removeWorktree: null }),
  openRemoveWorktree: (target) => set({ removeWorktree: target, createWorktreeFor: null }),
  close: () => set({ createWorktreeFor: null, removeWorktree: null }),
}))
