import { create } from "zustand"

import { useWorkspace } from "@/shared/workspace/workspace-context"

/** What the peek drawer is showing. Client-only UI state; it never reaches the backend. */
export interface PeekTarget {
  file: string
  line: number
  title: string
}

interface PeekState {
  /** Peek target per worktree tab. */
  targets: Record<string, PeekTarget | undefined>
  open: (worktreeId: string, target: PeekTarget) => void
  close: (worktreeId: string) => void
}

export const usePeekStore = create<PeekState>((set) => ({
  targets: {},
  open: (worktreeId, target) => set((s) => ({ targets: { ...s.targets, [worktreeId]: target } })),
  close: (worktreeId) => set((s) => ({ targets: { ...s.targets, [worktreeId]: undefined } })),
}))

/** Peek state for the current worktree tab. */
export function usePeek() {
  const { id } = useWorkspace()
  const target = usePeekStore((s) => s.targets[id])
  const open = usePeekStore((s) => s.open)
  const close = usePeekStore((s) => s.close)
  return { target, open: (t: PeekTarget) => open(id, t), close: () => close(id) }
}
