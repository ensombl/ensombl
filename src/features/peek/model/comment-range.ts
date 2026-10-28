import type { AnchoredComment } from "@/shared/bindings/AnchoredComment"

/** The lines a comment covers now; `null` when its code is gone. */
export function currentRange({ comment, currentLine }: AnchoredComment): { start: number; end: number } | null {
  if (currentLine === null) return null
  return { start: currentLine, end: currentLine + Math.max(0, (comment.endLine ?? comment.line) - comment.line) }
}

/** `3` or `3-7`. */
export const rangeLabel = (start: number, end: number) => (end > start ? `${start}-${end}` : String(start))

/** An in-progress drag over line numbers, in either direction. */
export const ordered = (a: number, b: number) => ({ start: Math.min(a, b), end: Math.max(a, b) })
