import { useQuery } from "@tanstack/react-query"

import type { SessionView } from "@/shared/bindings/SessionView"
import type { WorktreeListing } from "@/shared/bindings/WorktreeListing"
import { query } from "@/shared/ipc/client"

import { workspaceKeys } from "./keys"

export const getSession = () => query<SessionView>("get_session")

export function useSession() {
  return useQuery({ queryKey: workspaceKeys.session(), queryFn: getSession })
}

export function useWorktrees(projectId: string, enabled = true) {
  return useQuery({
    queryKey: workspaceKeys.worktrees(projectId),
    queryFn: () => query<WorktreeListing[]>("list_worktrees", { input: { projectId } }),
    enabled,
  })
}

export function useBranches(projectId: string, enabled = true) {
  return useQuery({
    queryKey: workspaceKeys.branches(projectId),
    queryFn: () => query<string[]>("list_branches", { input: { projectId } }),
    enabled,
  })
}
