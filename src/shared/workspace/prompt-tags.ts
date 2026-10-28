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
  add: (worktreeId: string, tag: PromptTag) => void
  remove: (worktreeId: string, key: string) => void
  clear: (worktreeId: string) => void
}

const usePromptTagsStore = create<PromptTagsStore>((set) => ({
  byWorktree: {},
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
  return {
    tags,
    add: (tag: PromptTag) => add(id, tag),
    remove: (key: string) => remove(id, key),
    clear: () => clear(id),
  }
}
