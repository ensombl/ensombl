import { create } from "zustand"

import { useWorkspace } from "@/shared/workspace/workspace-context"

/** Symbols changed on disk (usually by an agent) that you haven't looked at yet. Per worktree. */
interface ChangesStore {
  byWorktree: Record<string, Record<string, number>>
  mark: (worktreeId: string, ids: string[]) => void
  ack: (worktreeId: string, ids: string[]) => void
  clear: (worktreeId: string) => void
}

export const useChangesStore = create<ChangesStore>((set) => ({
  byWorktree: {},
  mark: (worktreeId, ids) =>
    set((s) => {
      const now = Date.now()
      const next = { ...(s.byWorktree[worktreeId] ?? {}) }
      for (const id of ids) next[id] = now
      return { byWorktree: { ...s.byWorktree, [worktreeId]: next } }
    }),
  ack: (worktreeId, ids) =>
    set((s) => {
      const current = s.byWorktree[worktreeId]
      if (!current || !ids.some((id) => id in current)) return s
      const next = { ...current }
      for (const id of ids) delete next[id]
      return { byWorktree: { ...s.byWorktree, [worktreeId]: next } }
    }),
  clear: (worktreeId) => set((s) => ({ byWorktree: { ...s.byWorktree, [worktreeId]: {} } })),
}))

const EMPTY: Record<string, number> = {}

export function useChanges() {
  const { id } = useWorkspace()
  const changed = useChangesStore((s) => s.byWorktree[id]) ?? EMPTY
  const ack = useChangesStore((s) => s.ack)
  const clear = useChangesStore((s) => s.clear)
  return {
    /** Symbol id to when it last changed (ms). */
    changed,
    ack: (ids: string[]) => ack(id, ids),
    clear: () => clear(id),
  }
}
