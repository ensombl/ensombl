import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import type { ComponentOverride } from "@/shared/bindings/ComponentOverride"
import { command } from "@/shared/ipc/client"

import { systemKeys } from "../queries/system-queries"

/** Edits to `.crm/architecture.json`; each refetches the system view. */
export function useArchitectureCommands(worktreeId: string) {
  const client = useQueryClient()
  const options = {
    onSuccess: () => client.invalidateQueries({ queryKey: systemKeys.all }),
    onError: (error: unknown) => toast.error("Couldn't update the architecture", { description: String(error) }),
  }
  return {
    update: useMutation({
      mutationFn: ({ componentId, value }: { componentId: string; value: ComponentOverride }) =>
        command("update_component", { input: { worktreeId, componentId, value } }),
      ...options,
    }),
    merge: useMutation({
      mutationFn: ({ componentIds, name }: { componentIds: string[]; name: string }) =>
        command<string>("merge_components", { input: { worktreeId, componentIds, name } }),
      ...options,
    }),
    split: useMutation({
      mutationFn: (componentId: string) => command("split_component", { input: { worktreeId, componentId } }),
      ...options,
    }),
    setRule: useMutation({
      mutationFn: ({ rule, enabled }: { rule: string; enabled: boolean }) =>
        command("set_rule_enabled", { input: { worktreeId, rule, enabled } }),
      ...options,
    }),
  }
}
