import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { PatternInstance } from "@/shared/bindings/PatternInstance"
import type { SymbolKind } from "@/shared/bindings/SymbolKind"
import type { PromptTag } from "@/shared/workspace/prompt-tags"

export interface Suggestion {
  tag: PromptTag
  /** Secondary text, e.g. the file. */
  detail: string
  kind: SymbolKind | "member" | "file" | "pattern"
}

/** Everything taggable in a model: symbols, their members, and files. */
export function taggables(model: ArchModel | null, patterns: PatternInstance[] = []): Suggestion[] {
  if (!model) return []
  const out: Suggestion[] = patterns.map((p) => ({
    tag: { type: "pattern", patternId: p.id, label: `pattern:${p.name.replace(": ", "/")}` },
    detail: `${p.participants.length} participants`,
    kind: "pattern",
  }))
  const files = new Set<string>()
  for (const s of model.symbols) {
    out.push({ tag: { type: "symbol", symbolId: s.id, label: s.name }, detail: s.file, kind: s.kind })
    files.add(s.file)
  }
  for (const s of model.symbols) {
    for (const m of s.members) {
      if (m.kind === "enumMember") continue
      out.push({
        tag: { type: "member", symbolId: s.id, member: m.name, label: `${s.name}.${m.name}` },
        detail: m.signature,
        kind: "member",
      })
    }
  }
  for (const f of [...files].sort()) {
    out.push({ tag: { type: "file", path: f, label: f.slice(f.lastIndexOf("/") + 1) }, detail: f, kind: "file" })
  }
  return out
}

/** Ranks prefix matches first, then substring matches; members only once a `.` is typed. */
export function matchSuggestions(all: Suggestion[], query: string, limit = 8): Suggestion[] {
  const q = query.toLowerCase()
  const wantsMember = q.includes(".")
  const scored: [number, Suggestion][] = []
  for (const s of all) {
    if (s.kind === "member" && !wantsMember) continue
    const label = s.tag.label.toLowerCase()
    const score = label.startsWith(q) ? 0 : label.includes(q) ? 1 : s.detail.toLowerCase().includes(q) ? 2 : -1
    if (score >= 0) scored.push([score * 1000 + label.length, s])
    if (scored.length > 2000) break
  }
  return scored.sort((a, b) => a[0] - b[0]).slice(0, limit).map(([, s]) => s)
}

/** The `@query` being typed right before the caret, if any. */
export function mentionAt(text: string, caret: number): { start: number; query: string } | null {
  const match = /(^|\s)@([\w.$/:-]*)$/.exec(text.slice(0, caret))
  if (!match) return null
  return { start: caret - match[2].length - 1, query: match[2] }
}
