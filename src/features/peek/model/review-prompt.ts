import type { AnchoredComment } from "@/shared/bindings/AnchoredComment"

/** Turns open review comments into an instruction for an agent. */
export function reviewPrompt(comments: AnchoredComment[], sources: Record<string, string | undefined> = {}): string {
  const open = comments.filter((c) => !c.comment.resolved)
  if (open.length === 0) return ""
  const items = open.map(({ comment, currentLine }, i) => {
    const line = currentLine ?? comment.line
    const code = sources[comment.file]?.split("\n")[line - 1]?.trim() ?? comment.anchor
    const where = currentLine === null ? `@${comment.file} (code has changed since)` : `@${comment.file}:${line}`
    return `${i + 1}. ${where}\n   > ${code}\n   ${comment.body.split("\n").join("\n   ")}`
  })
  return `Please address these review comments:\n\n${items.join("\n\n")}`
}
