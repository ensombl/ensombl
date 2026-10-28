import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { EdgeKind } from "@/shared/bindings/EdgeKind"
import type { FlowStep } from "@/shared/bindings/FlowStep"
import type { Interaction } from "@/shared/bindings/Interaction"
import type { PatternInstance } from "@/shared/bindings/PatternInstance"
import type { Member } from "@/shared/bindings/Member"
import type { SymbolNode } from "@/shared/bindings/SymbolNode"
import type { SystemModel } from "@/shared/bindings/SystemModel"
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

function describePattern(model: ArchModel | null, pattern: PatternInstance): string {
  const byId = new Map(model?.symbols.map((s) => [s.id, s]))
  const lines = [
    `### pattern ${pattern.name} (${pattern.kind}, confidence ${pattern.confidence.toFixed(2)})`,
    `evidence: ${pattern.evidence.join("; ")}`,
    "roles:",
  ]
  for (const p of pattern.participants) {
    const s = byId.get(p.symbolId)
    lines.push(s ? `  - ${p.role}: ${s.name}  @${s.file}:${s.range.startLine}` : `  - ${p.role}: ${p.symbolId}`)
  }
  return lines.join("\n")
}

const MAX_FLOW_STEPS = 60
const INTERACTION_VERB: Record<Interaction["kind"], string> = {
  call: "calls",
  inject: "injects",
  emits: "emits",
  handles: "delivers to",
  http: "requests",
  renders: "renders",
}

function describeComponent(system: SystemModel, id: string): string | null {
  const node = system.nodes.find((n) => n.id === id)
  if (!node) return null
  const name = (nid: string) => system.nodes.find((n) => n.id === nid)?.name ?? nid
  const relation = (i: Interaction, other: string) =>
    `${name(other)} (${INTERACTION_VERB[i.kind]}${i.members.length ? ` ${i.members.slice(0, 6).join(", ")}` : ""})`
  const lines = [`### component ${node.name} (${node.role}, ${node.layer} layer, feature ${node.feature})`]
  if (node.description) lines.push(`description: ${node.description}`)
  if (node.file) lines.push(`@${node.file}${node.line ? `:${node.line}` : ""}`)
  if (node.merged.length) lines.push(`made of: ${node.merged.map((m) => m.split("#").pop()).join(", ")}`)
  const out = system.interactions.filter((i) => i.from === id).map((i) => relation(i, i.to))
  const inc = system.interactions.filter((i) => i.to === id).map((i) => relation(i, i.from))
  if (out.length) lines.push(`talks to: ${out.slice(0, MAX_RELATED).join("; ")}`)
  if (inc.length) lines.push(`used by: ${inc.slice(0, MAX_RELATED).join("; ")}`)
  const entries = system.entryPoints.filter((e) => e.node === id).map((e) => e.label)
  if (entries.length) lines.push(`entry points: ${entries.slice(0, MAX_RELATED).join(", ")}`)
  return lines.join("\n")
}

function describeFlow(system: SystemModel, entryId: string, steps: FlowStep[] | undefined): string | null {
  const entry = system.entryPoints.find((e) => e.id === entryId)
  if (!entry) return null
  const name = (nid: string) => system.nodes.find((n) => n.id === nid)?.name ?? nid
  const lines = [
    `### flow ${entry.label} (starts in ${name(entry.node)}${entry.member ? `.${entry.member}` : ""} @${entry.site.file}:${entry.site.line})`,
  ]
  for (const s of (steps ?? []).slice(0, MAX_FLOW_STEPS)) {
    lines.push(`${"  ".repeat(s.depth)}- ${name(s.from)} → ${name(s.to)}: ${s.label}${s.isAsync ? " (async)" : ""}  @${s.site.file}:${s.site.line}`)
  }
  if ((steps?.length ?? 0) > MAX_FLOW_STEPS) lines.push(`… ${steps!.length - MAX_FLOW_STEPS} more steps`)
  return lines.join("\n")
}

/** What tags need beyond the code model. */
export interface TagContext {
  patterns?: PatternInstance[]
  system?: SystemModel | null
  /** Traced steps per tagged flow's entry id. */
  flows?: Record<string, FlowStep[]>
}

function describeTag(model: ArchModel | null, tag: PromptTag, { patterns = [], system, flows }: TagContext): string | null {
  if (tag.type === "file") return `### file\n@${tag.path}`
  if (tag.type === "component") return system ? describeComponent(system, tag.componentId) : null
  if (tag.type === "flow") return system ? describeFlow(system, tag.entryId, flows?.[tag.entryId]) : null
  if (tag.type === "pattern") {
    const pattern = patterns.find((p) => p.id === tag.patternId)
    return pattern ? describePattern(model, pattern) : null
  }
  const symbol = model?.symbols.find((s) => s.id === tag.symbolId)
  if (!model || !symbol) return null
  if (tag.type === "symbol") return describeSymbol(model, symbol)
  const member = symbol.members.find((m) => m.name === tag.member)
  if (!member) return null
  return [`### ${symbol.name}.${member.name}`, `@${symbol.file}:${member.line}`, memberLine(member).trim()].join("\n")
}

/** The full text sent to the agent: the user's prompt followed by tag context. */
export function buildPrompt(text: string, tags: PromptTag[], model: ArchModel | null, context: TagContext = {}): string {
  const blocks = tags.map((t) => describeTag(model, t, context)).filter((b): b is string => b !== null)
  if (blocks.length === 0) return text.trim()
  return `${text.trim()}\n\n<architecture-context source="ensombl">\n${blocks.join("\n\n")}\n</architecture-context>`
}

/** Wraps text in bracketed-paste markers so multi-line prompts don't submit early. */
export const bracketedPaste = (text: string) => `\x1b[200~${text}\x1b[201~`
