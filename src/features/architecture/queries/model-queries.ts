import { useQuery } from "@tanstack/react-query"

import type { ArchModel } from "@/shared/bindings/ArchModel"
import { query } from "@/shared/ipc/client"

export const modelKeys = {
  all: ["model"] as const,
  model: (worktreeId: string) => [...modelKeys.all, worktreeId] as const,
}

export function useModel(worktreeId: string) {
  return useQuery({
    queryKey: modelKeys.model(worktreeId),
    queryFn: () => query<ArchModel | null>("get_model", { input: { worktreeId } }),
  })
}
