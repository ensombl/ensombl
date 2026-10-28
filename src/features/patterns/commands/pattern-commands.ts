import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import type { SetPatternDecision } from "@/shared/bindings/SetPatternDecision"
import { command } from "@/shared/ipc/client"

import { patternKeys } from "../queries/pattern-queries"

export function useSetPatternDecision() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (input: SetPatternDecision) => command("set_pattern_decision", { input }),
    onSettled: (_, __, input) => client.invalidateQueries({ queryKey: patternKeys.list(input.worktreeId) }),
    onError: (error) => toast.error("Couldn't save decision", { description: String(error) }),
  })
}
