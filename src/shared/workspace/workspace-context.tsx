import { createContext, useContext, type ReactNode } from "react"

import type { Worktree } from "@/shared/bindings/Worktree"

const WorkspaceContext = createContext<Worktree | null>(null)

/** Scopes everything below it to one worktree tab. */
export function WorkspaceProvider({ worktree, children }: { worktree: Worktree; children: ReactNode }) {
  return <WorkspaceContext.Provider value={worktree}>{children}</WorkspaceContext.Provider>
}

export function useWorkspace(): Worktree {
  const worktree = useContext(WorkspaceContext)
  if (!worktree) throw new Error("useWorkspace must be used inside a WorkspaceProvider")
  return worktree
}
