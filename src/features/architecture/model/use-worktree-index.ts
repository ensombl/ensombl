import { useEffect, useState } from "react"
import { useQueryClient } from "@tanstack/react-query"

import type { IndexProgress } from "@/shared/bindings/IndexProgress"
import type { ModelUpdated } from "@/shared/bindings/ModelUpdated"
import { useBackendEvent } from "@/shared/ipc/events"

import { dropIndex, useIndexWorktree } from "../commands/indexing-commands"
import { modelKeys, useModel } from "../queries/model-queries"

/**
 * Keeps a worktree indexed while its tab is mounted: indexes on mount, follows
 * live updates from the file watcher, and frees the index on unmount.
 */
export function useWorktreeIndex(worktreeId: string) {
  const client = useQueryClient()
  const model = useModel(worktreeId)
  const index = useIndexWorktree()
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null)
  const { mutate } = index

  useEffect(() => {
    mutate({ worktreeId, force: false })
    return () => {
      void dropIndex({ worktreeId })
    }
  }, [worktreeId, mutate])

  useBackendEvent<IndexProgress>("index://progress", (p) => {
    if (p.worktreeId !== worktreeId) return
    setProgress(p.done >= p.total ? null : { done: p.done, total: p.total })
  })

  useBackendEvent<ModelUpdated>("index://updated", (u) => {
    if (u.worktreeId === worktreeId) client.invalidateQueries({ queryKey: modelKeys.model(worktreeId) })
  })

  return {
    model: model.data ?? null,
    summary: index.data,
    indexing: index.isPending,
    progress,
    reindex: () => mutate({ worktreeId, force: true }),
  }
}
