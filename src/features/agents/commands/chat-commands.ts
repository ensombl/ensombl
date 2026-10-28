import { Channel } from "@tauri-apps/api/core"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import type { ChatEvent } from "@/shared/bindings/ChatEvent"
import type { ChatInfo } from "@/shared/bindings/ChatInfo"
import type { ChatSnapshot } from "@/shared/bindings/ChatSnapshot"
import type { PermissionDecision } from "@/shared/bindings/PermissionDecision"
import type { PermissionMode } from "@/shared/bindings/PermissionMode"
import type { StartChat } from "@/shared/bindings/StartChat"
import { command } from "@/shared/ipc/client"

import { agentKeys } from "../queries/agent-queries"

function channelFor(onEvent: (event: ChatEvent) => void) {
  const channel = new Channel<ChatEvent>()
  channel.onmessage = onEvent
  return channel
}

/** Attaches a view: returns the transcript so far, then streams new events to `onEvent`. */
export const attachChat = (chatId: string, onEvent: (event: ChatEvent) => void) =>
  command<ChatSnapshot>("attach_chat", { input: { chatId }, output: channelFor(onEvent) })

export const sendChatMessage = (chatId: string, text: string) =>
  command("send_chat_message", { input: { chatId, text } })

export const respondPermission = (chatId: string, requestId: string, decision: PermissionDecision) =>
  command("respond_permission", { input: { chatId, requestId, decision } })

export const interruptChat = (chatId: string) => command("interrupt_chat", { input: { chatId } })

export const setChatPermissionMode = (chatId: string, mode: PermissionMode) =>
  command("set_chat_permission_mode", { input: { chatId, mode } })

export const setChatModel = (chatId: string, model: string) => command("set_chat_model", { input: { chatId, model } })

export function useStartChat() {
  const client = useQueryClient()
  return useMutation({
    // The view attaches itself and gets the transcript, so the start stream is ignored.
    mutationFn: (input: StartChat) => command<ChatInfo>("start_chat", { input, output: channelFor(() => {}) }),
    onSuccess: (info) => client.invalidateQueries({ queryKey: agentKeys.chats(info.worktreeId) }),
    onError: (error) => toast.error("Couldn't start agent", { description: String(error) }),
  })
}

export function useCloseChat(worktreeId: string) {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (chatId: string) => command("close_chat", { input: { chatId } }),
    onSettled: () => client.invalidateQueries({ queryKey: agentKeys.chats(worktreeId) }),
  })
}
