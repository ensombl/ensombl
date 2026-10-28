import { useQuery } from "@tanstack/react-query"

import type { AnchoredComment } from "@/shared/bindings/AnchoredComment"
import type { Settings } from "@/shared/bindings/Settings"
import type { SourceText } from "@/shared/bindings/SourceText"
import { query } from "@/shared/ipc/client"

export const peekKeys = {
  sources: (worktreeId: string) => ["source", worktreeId] as const,
  source: (worktreeId: string, path: string) => ["source", worktreeId, path] as const,
  comments: (worktreeId: string) => ["comments", worktreeId] as const,
  settings: () => ["settings"] as const,
}

export function useSource(worktreeId: string, path: string) {
  return useQuery({
    queryKey: peekKeys.source(worktreeId, path),
    queryFn: () => query<SourceText>("read_source", { input: { worktreeId, path } }),
  })
}

export function useComments(worktreeId: string) {
  return useQuery({
    queryKey: peekKeys.comments(worktreeId),
    queryFn: () => query<AnchoredComment[]>("list_comments", { input: { worktreeId } }),
  })
}

export function useSettings() {
  return useQuery({ queryKey: peekKeys.settings(), queryFn: () => query<Settings>("get_settings") })
}
