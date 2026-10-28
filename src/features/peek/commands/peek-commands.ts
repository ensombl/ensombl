import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { agentKeys } from "@/features/agents/queries/agent-queries"
import type { AddComment } from "@/shared/bindings/AddComment"
import type { DeleteComment } from "@/shared/bindings/DeleteComment"
import type { EditorLaunch } from "@/shared/bindings/EditorLaunch"
import type { OpenInEditor } from "@/shared/bindings/OpenInEditor"
import type { ResolveComment } from "@/shared/bindings/ResolveComment"
import type { SaveSource } from "@/shared/bindings/SaveSource"
import type { Settings } from "@/shared/bindings/Settings"
import { command } from "@/shared/ipc/client"

import { peekKeys } from "../queries/peek-queries"

function useCommentCommand<I extends { worktreeId: string }>(name: string, errorTitle: string) {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (input: I) => command(name, { input }),
    onSettled: (_, __, input) => client.invalidateQueries({ queryKey: peekKeys.comments(input.worktreeId) }),
    onError: (error) => toast.error(errorTitle, { description: String(error) }),
  })
}

export const useAddComment = () => useCommentCommand<AddComment>("add_comment", "Couldn't add comment")
export const useResolveComment = () => useCommentCommand<ResolveComment>("resolve_comment", "Couldn't update comment")
export const useDeleteComment = () => useCommentCommand<DeleteComment>("delete_comment", "Couldn't delete comment")

export function useUpdateSettings() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (settings: Settings) => command("update_settings", { input: { settings } }),
    onSuccess: () => client.invalidateQueries({ queryKey: peekKeys.settings() }),
    onError: (error) => toast.error("Couldn't save settings", { description: String(error) }),
  })
}

export function useOpenInEditor() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (input: OpenInEditor) => command<EditorLaunch>("open_in_editor", { input }),
    onSuccess: (launch, input) => {
      if (launch.type === "terminal") client.invalidateQueries({ queryKey: agentKeys.terminals(input.worktreeId) })
    },
    onError: (error) => toast.error("Couldn't open editor", { description: String(error) }),
  })
}

/** Writes an edited file; refused when it changed on disk since `base`. */
export function useSaveSource() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (input: SaveSource) => command("save_source", { input }),
    onSuccess: (_, input) => client.setQueryData(peekKeys.source(input.worktreeId, input.path), { path: input.path, content: input.content }),
  })
}
