import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import type { ActivateTab } from "@/shared/bindings/ActivateTab"
import type { AddProject } from "@/shared/bindings/AddProject"
import type { CloseTab } from "@/shared/bindings/CloseTab"
import type { CreateWorktree } from "@/shared/bindings/CreateWorktree"
import type { OpenWorktree } from "@/shared/bindings/OpenWorktree"
import type { RemoveProject } from "@/shared/bindings/RemoveProject"
import type { RemoveWorktree } from "@/shared/bindings/RemoveWorktree"
import type { ToggleGroup } from "@/shared/bindings/ToggleGroup"
import { command } from "@/shared/ipc/client"

import { workspaceKeys } from "../queries/keys"

/** Builds a mutation hook for a backend command; every command refreshes the workspace read models. */
function workspaceCommand<I, O = void>(name: string, errorTitle: string) {
  return function useWorkspaceCommand() {
    const client = useQueryClient()
    return useMutation({
      mutationFn: (input: I) => command<O>(name, { input }),
      onSettled: () => client.invalidateQueries({ queryKey: workspaceKeys.all }),
      onError: (error) => toast.error(errorTitle, { description: String(error) }),
    })
  }
}

export const useAddProject = workspaceCommand<AddProject, string>("add_project", "Couldn't add project")
export const useCreateWorktree = workspaceCommand<CreateWorktree, string>("create_worktree", "Couldn't create worktree")
export const useOpenWorktree = workspaceCommand<OpenWorktree, string>("open_worktree", "Couldn't open worktree")
export const useRemoveWorktree = workspaceCommand<RemoveWorktree>("remove_worktree", "Couldn't delete worktree")
export const useCloseTab = workspaceCommand<CloseTab>("close_tab", "Couldn't close tab")
export const useActivateTab = workspaceCommand<ActivateTab>("activate_tab", "Couldn't switch tab")
export const useToggleGroup = workspaceCommand<ToggleGroup>("toggle_group", "Couldn't toggle group")
export const useRemoveProject = workspaceCommand<RemoveProject>("remove_project", "Couldn't remove project")
