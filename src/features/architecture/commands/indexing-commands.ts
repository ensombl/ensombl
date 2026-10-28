import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import type { DropIndex } from "@/shared/bindings/DropIndex"
import type { IndexSummary } from "@/shared/bindings/IndexSummary"
import type { IndexWorktree } from "@/shared/bindings/IndexWorktree"
import { command } from "@/shared/ipc/client"

import { modelKeys } from "../queries/model-queries"

export const indexWorktree = (input: IndexWorktree) => command<IndexSummary>("index_worktree", { input })
export const dropIndex = (input: DropIndex) => command("drop_index", { input })

export function useIndexWorktree() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: indexWorktree,
    onSuccess: (_, input) => client.invalidateQueries({ queryKey: modelKeys.model(input.worktreeId) }),
    onError: (error) => toast.error("Indexing failed", { description: String(error) }),
  })
}
