import { useQuery, useQueryClient } from "@tanstack/react-query"

import type { ModelUpdated } from "@/shared/bindings/ModelUpdated"
import type { PatternView } from "@/shared/bindings/PatternView"
import { useBackendEvent } from "@/shared/ipc/events"
import { query } from "@/shared/ipc/client"

export const patternKeys = {
  all: ["patterns"] as const,
  list: (worktreeId: string) => [...patternKeys.all, worktreeId] as const,
}

/** Detected patterns for a worktree; refreshed whenever its model changes. */
export function usePatterns(worktreeId: string, enabled = true) {
  const client = useQueryClient()
  useBackendEvent<ModelUpdated>("index://updated", (u) => {
    if (u.worktreeId === worktreeId) client.invalidateQueries({ queryKey: patternKeys.list(worktreeId) })
  })
  return useQuery({
    queryKey: patternKeys.list(worktreeId),
    queryFn: () => query<PatternView[]>("get_patterns", { input: { worktreeId } }),
    enabled,
  })
}
