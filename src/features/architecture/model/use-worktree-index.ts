import { useEffect, useRef, useState } from "react"
import { useQueryClient } from "@tanstack/react-query"

import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { IndexProgress } from "@/shared/bindings/IndexProgress"
import type { ModelUpdated } from "@/shared/bindings/ModelUpdated"
import { useBackendEvent } from "@/shared/ipc/events"

import { peekKeys } from "@/features/peek/queries/peek-queries"

import { dropIndex, useIndexWorktree } from "../commands/indexing-commands"
import { applyDelta } from "./apply-delta"
import { useChangesStore } from "./changes-store"
import { modelKeys, useModel } from "../queries/model-queries"
import { systemKeys } from "../queries/system-queries"

const SYSTEM_REFETCH_DELAY = 1000

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

  const markChanged = useChangesStore((s) => s.mark)
  // Agents save files in bursts; refetch the (large) system view once things settle.
  const systemRefetch = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  useEffect(() => () => clearTimeout(systemRefetch.current), [])
  useBackendEvent<ModelUpdated>("index://updated", (u) => {
    if (u.worktreeId !== worktreeId) return
    // The system view is derived on the backend; refetch it after a quiet moment.
    clearTimeout(systemRefetch.current)
    systemRefetch.current = setTimeout(() => client.invalidateQueries({ queryKey: systemKeys.system(worktreeId) }), SYSTEM_REFETCH_DELAY)
    // Open files may have been edited by an agent.
    client.invalidateQueries({ queryKey: peekKeys.sources(worktreeId) })
    const key = modelKeys.model(worktreeId)
    const current = client.getQueryData<ArchModel | null>(key)
    if (!u.delta || !current) {
      client.invalidateQueries({ queryKey: key })
      return
    }
    // Patch in place: the full model can be megabytes on big repos.
    client.setQueryData(key, applyDelta(current, u.delta))
    if (u.delta.upserted.length > 0) markChanged(worktreeId, u.delta.upserted.map((s) => s.id))
  })

  return {
    model: model.data ?? null,
    summary: index.data,
    indexing: index.isPending,
    progress,
    reindex: () => mutate({ worktreeId, force: true }),
  }
}
