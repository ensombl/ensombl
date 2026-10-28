import type { AnchoredComment } from "@/shared/bindings/AnchoredComment"

import { currentRange, rangeLabel } from "./comment-range"

/** Turns open review comments into an instruction for an agent. */
export function reviewPrompt(comments: AnchoredComment[], sources: Record<string, string | undefined> = {}): string {
  const open = comments.filter((c) => !c.comment.resolved)
  if (open.length === 0) return ""
  const items = open.map((item, i) => {
    const { comment } = item
    const range = currentRange(item)
    const lines = sources[comment.file]?.split("\n")
    const code = range && lines ? lines.slice(range.start - 1, range.end).map((l) => l.trim()) : [comment.anchor]
    const where = range ? `@${comment.file}:${rangeLabel(range.start, range.end)}` : `@${comment.file} (code has changed since)`
    return `${i + 1}. ${where}\n${code.map((l) => `   > ${l}`).join("\n")}\n   ${comment.body.split("\n").join("\n   ")}`
  })
  return `Please address these review comments:\n\n${items.join("\n\n")}`
}
