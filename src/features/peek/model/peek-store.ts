import { create } from "zustand"

/** What the peek drawer is showing. Client-only UI state; it never reaches the backend. */
export interface PeekTarget {
  file: string
  line: number
  title: string
}

interface PeekState {
  target: PeekTarget | null
  open: (target: PeekTarget) => void
  close: () => void
}

export const usePeekStore = create<PeekState>((set) => ({
  target: null,
  open: (target) => set({ target }),
  close: () => set({ target: null }),
}))
