import { createContext, useContext, type ReactNode } from "react"

import type { Worktree } from "@/shared/bindings/Worktree"

interface WorkspaceValue {
  worktree: Worktree
  /** Whether this tab is the visible one (hidden tabs stay mounted). */
  active: boolean
}

const WorkspaceContext = createContext<WorkspaceValue | null>(null)

/** Scopes everything below it to one worktree tab. */
export function WorkspaceProvider({ worktree, active, children }: WorkspaceValue & { children: ReactNode }) {
  return <WorkspaceContext.Provider value={{ worktree, active }}>{children}</WorkspaceContext.Provider>
}

function useWorkspaceValue(): WorkspaceValue {
  const value = useContext(WorkspaceContext)
  if (!value) throw new Error("useWorkspace must be used inside a WorkspaceProvider")
  return value
}

export function useWorkspace(): Worktree {
  return useWorkspaceValue().worktree
}

export function useIsActiveWorkspace(): boolean {
  return useWorkspaceValue().active
}
