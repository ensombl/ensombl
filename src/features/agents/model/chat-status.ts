import { create } from "zustand"

/** Live status per chat, published by its view so tabs can show it. */
export interface ChatStatus {
  busy: boolean
  running: boolean
  /** Waiting on the user to approve something. */
  needsInput: boolean
}

interface ChatStatusStore {
  byChat: Record<string, ChatStatus>
  set: (chatId: string, status: ChatStatus) => void
}

export const useChatStatus = create<ChatStatusStore>((set) => ({
  byChat: {},
  set: (chatId, status) => set((s) => ({ byChat: { ...s.byChat, [chatId]: status } })),
}))
