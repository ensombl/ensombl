import { create } from "zustand"

import { useWorkspace } from "./workspace-context"

/** Something tagged into an agent prompt, like a file mention but architectural. */
export type PromptTag =
  | { type: "symbol"; symbolId: string; label: string }
  | { type: "member"; symbolId: string; member: string; label: string }
  | { type: "file"; path: string; label: string }
  | { type: "pattern"; patternId: string; label: string }

export function tagKey(tag: PromptTag): string {
  switch (tag.type) {
    case "file":
      return `file:${tag.path}`
    case "member":
      return `member:${tag.symbolId}.${tag.member}`
    case "pattern":
      return `pattern:${tag.patternId}`
    default:
      return `symbol:${tag.symbolId}`
  }
}

interface PromptTagsStore {
  byWorktree: Record<string, PromptTag[]>
  /** Text queued for the composer (e.g. a review), consumed once. */
  drafts: Record<string, string>
  appendDraft: (worktreeId: string, text: string) => void
  takeDraft: (worktreeId: string) => string
  add: (worktreeId: string, tag: PromptTag) => void
  remove: (worktreeId: string, key: string) => void
  clear: (worktreeId: string) => void
}

const usePromptTagsStore = create<PromptTagsStore>((set, get) => ({
  byWorktree: {},
  drafts: {},
  appendDraft: (id, text) =>
    set((s) => ({ drafts: { ...s.drafts, [id]: s.drafts[id] ? `${s.drafts[id]}\n\n${text}` : text } })),
  takeDraft: (id) => {
    const text = get().drafts[id] ?? ""
    if (text) set((s) => ({ drafts: { ...s.drafts, [id]: "" } }))
    return text
  },
  add: (id, tag) =>
    set((s) => {
      const list = s.byWorktree[id] ?? []
      if (list.some((t) => tagKey(t) === tagKey(tag))) return s
      return { byWorktree: { ...s.byWorktree, [id]: [...list, tag] } }
    }),
  remove: (id, key) =>
    set((s) => ({ byWorktree: { ...s.byWorktree, [id]: (s.byWorktree[id] ?? []).filter((t) => tagKey(t) !== key) } })),
  clear: (id) => set((s) => ({ byWorktree: { ...s.byWorktree, [id]: [] } })),
}))

const EMPTY: PromptTag[] = []

/** Tags pending in the current worktree's agent prompt. Shared so the diagram can tag into the composer. */
export function usePromptTags() {
  const { id } = useWorkspace()
  const tags = usePromptTagsStore((s) => s.byWorktree[id]) ?? EMPTY
  const add = usePromptTagsStore((s) => s.add)
  const remove = usePromptTagsStore((s) => s.remove)
  const clear = usePromptTagsStore((s) => s.clear)
  const draft = usePromptTagsStore((s) => s.drafts[id] ?? "")
  const appendDraft = usePromptTagsStore((s) => s.appendDraft)
  const takeDraft = usePromptTagsStore((s) => s.takeDraft)
  return {
    tags,
    draft,
    appendDraft: (text: string) => appendDraft(id, text),
    takeDraft: () => takeDraft(id),
    add: (tag: PromptTag) => add(id, tag),
    remove: (key: string) => remove(id, key),
    clear: () => clear(id),
  }
}
