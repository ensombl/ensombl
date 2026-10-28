import { useQuery } from "@tanstack/react-query"

import type { FlowStep } from "@/shared/bindings/FlowStep"
import type { SystemView } from "@/shared/bindings/SystemView"
import { query } from "@/shared/ipc/client"

export const systemKeys = {
  all: ["system"] as const,
  system: (worktreeId: string) => [...systemKeys.all, worktreeId] as const,
  flow: (worktreeId: string, entryId: string, depth: number) => [...systemKeys.all, worktreeId, "flow", entryId, depth] as const,
}

/** Components and interactions with the user's corrections; `null` until indexed. */
export function useSystem(worktreeId: string, enabled = true) {
  return useQuery({
    queryKey: systemKeys.system(worktreeId),
    queryFn: () => query<SystemView | null>("get_system", { input: { worktreeId } }),
    enabled,
  })
}

export function useFlow(worktreeId: string, entryId: string | null, depth: number) {
  return useQuery({
    queryKey: systemKeys.flow(worktreeId, entryId ?? "", depth),
    queryFn: () => query<FlowStep[]>("get_flow", { input: { worktreeId, entryId, depth } }),
    enabled: !!entryId,
  })
}
