import { createContext, useContext } from "react"

import type { PermissionDecision } from "@/shared/bindings/PermissionDecision"
import type { PermissionMode } from "@/shared/bindings/PermissionMode"

/** What rows in a chat can do, provided by the chat view. */
export interface ChatActions {
  cwd: string
  respond: (requestId: string, decision: PermissionDecision) => void
  setMode: (mode: PermissionMode) => void
  openFile: (file: string, line?: number) => void
}

export const ChatActionsContext = createContext<ChatActions | null>(null)

export function useChatActions(): ChatActions {
  const value = useContext(ChatActionsContext)
  if (!value) throw new Error("useChatActions must be used inside a chat view")
  return value
}
