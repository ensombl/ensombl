import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { EdgeKind } from "@/shared/bindings/EdgeKind"
import type { Member } from "@/shared/bindings/Member"
import type { SymbolNode } from "@/shared/bindings/SymbolNode"
import type { PromptTag } from "@/shared/workspace/prompt-tags"

/**
 * Expands prompt tags into compact architectural context the agent can act
 * on: where a symbol lives, what it inherits, its signatures and relations.
 */

const VISIBILITY = { public: "+", protected: "#", private: "-" } as const
const RELATION_LABEL: Record<EdgeKind, string> = {
  extends: "extends",
  implements: "implements",
  association: "has",
  dependency: "uses",
}
const MAX_MEMBERS = 40
const MAX_RELATED = 12

const memberLine = (m: Member) =>
  `  ${VISIBILITY[m.visibility]} ${m.isStatic ? "static " : ""}${m.isAsync ? "async " : ""}${m.signature}  (line ${m.line})`

function relations(model: ArchModel, symbol: SymbolNode): string[] {
  const names = new Map(model.symbols.map((s) => [s.id, s.name]))
  const out = new Map<EdgeKind, string[]>()
  const usedBy: string[] = []
  for (const e of model.edges) {
    if (e.from === symbol.id && names.has(e.to)) out.set(e.kind, [...(out.get(e.kind) ?? []), names.get(e.to)!])
    else if (e.to === symbol.id && names.has(e.from)) usedBy.push(names.get(e.from)!)
  }
  const lines: string[] = []
  for (const kind of ["extends", "implements", "association", "dependency"] as EdgeKind[]) {
    const list = out.get(kind)
    if (list?.length) lines.push(`${RELATION_LABEL[kind]}: ${list.slice(0, MAX_RELATED).join(", ")}${list.length > MAX_RELATED ? ", …" : ""}`)
  }
  const unique = [...new Set(usedBy)]
  if (unique.length) lines.push(`used by: ${unique.slice(0, MAX_RELATED).join(", ")}${unique.length > MAX_RELATED ? ", …" : ""}`)
  return lines
}

function describeSymbol(model: ArchModel, symbol: SymbolNode): string {
  const lines = [
    `### ${symbol.kind === "typeAlias" ? "type" : symbol.kind} ${symbol.name}${symbol.typeParams ?? ""}`,
    `@${symbol.file}:${symbol.range.startLine}-${symbol.range.endLine} (package ${symbol.package})`,
    symbol.signature,
    ...relations(model, symbol),
  ]
  if (symbol.members.length > 0) {
    lines.push("members:")
    lines.push(...symbol.members.slice(0, MAX_MEMBERS).map(memberLine))
    if (symbol.members.length > MAX_MEMBERS) lines.push(`  … ${symbol.members.length - MAX_MEMBERS} more`)
  }
  return lines.join("\n")
}

function describeTag(model: ArchModel | null, tag: PromptTag): string | null {
  if (tag.type === "file") return `### file\n@${tag.path}`
  const symbol = model?.symbols.find((s) => s.id === tag.symbolId)
  if (!model || !symbol) return null
  if (tag.type === "symbol") return describeSymbol(model, symbol)
  const member = symbol.members.find((m) => m.name === tag.member)
  if (!member) return null
  return [`### ${symbol.name}.${member.name}`, `@${symbol.file}:${member.line}`, memberLine(member).trim()].join("\n")
}

/** The full text sent to the agent: the user's prompt followed by tag context. */
export function buildPrompt(text: string, tags: PromptTag[], model: ArchModel | null): string {
  const blocks = tags.map((t) => describeTag(model, t)).filter((b): b is string => b !== null)
  if (blocks.length === 0) return text.trim()
  return `${text.trim()}\n\n<architecture-context source="CRM">\n${blocks.join("\n\n")}\n</architecture-context>`
}

/** Wraps text in bracketed-paste markers so multi-line prompts don't submit early. */
export const bracketedPaste = (text: string) => `\x1b[200~${text}\x1b[201~`
