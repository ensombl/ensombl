import { create } from "zustand"

interface Draft {
  content: string
  /** The file as it was on disk when editing started. */
  base: string
}

/**
 * Unsaved edits from the Code tab, by `worktreeId:path`. Drafts outlive the
 * editor so switching files doesn't throw edits away. Client-only.
 */
interface DraftsStore {
  drafts: Record<string, Draft>
  /** Files with unsaved edits, including the one open now. */
  dirty: Record<string, true>
  keep: (key: string, draft: Draft | null) => void
  setDirty: (key: string, dirty: boolean) => void
}

const without = <T,>(record: Record<string, T>, key: string) => {
  const next = { ...record }
  delete next[key]
  return next
}

export const useDraftsStore = create<DraftsStore>((set) => ({
  drafts: {},
  dirty: {},
  keep: (key, draft) => set((s) => ({ drafts: draft ? { ...s.drafts, [key]: draft } : without(s.drafts, key) })),
  setDirty: (key, dirty) =>
    set((s) => (dirty === !!s.dirty[key] ? s : { dirty: dirty ? { ...s.dirty, [key]: true } : without(s.dirty, key) })),
}))

export const draftKey = (worktreeId: string, path: string) => `${worktreeId}:${path}`
