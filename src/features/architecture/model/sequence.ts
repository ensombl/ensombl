import type { EntryPoint } from "@/shared/bindings/EntryPoint"
import type { FlowStep } from "@/shared/bindings/FlowStep"

/** A traced flow arranged as a sequence diagram: lifelines left to right, one row per step. */
export interface Sequence {
  /** Node ids in order of first appearance, starting with the entry point's. */
  participants: string[]
  rows: (FlowStep & { fromIndex: number; toIndex: number })[]
}

export function toSequence(entryNode: string, steps: FlowStep[]): Sequence {
  const participants = [entryNode]
  const indexOf = (id: string) => {
    let i = participants.indexOf(id)
    if (i < 0) i = participants.push(id) - 1
    return i
  }
  const rows = steps.map((s) => ({ ...s, fromIndex: indexOf(s.from), toIndex: indexOf(s.to) }))
  return { participants, rows }
}

export const ENTRY_GROUPS: { kind: EntryPoint["kind"]; label: string }[] = [
  { kind: "http", label: "HTTP" },
  { kind: "page", label: "Pages" },
  { kind: "event", label: "Events" },
  { kind: "queue", label: "Queues" },
  { kind: "ws", label: "WebSocket" },
  { kind: "cron", label: "Scheduled" },
]

/** Entry points matching `query`, grouped by kind in display order. */
export function groupEntries(entries: EntryPoint[], query: string, nameOf: (id: string) => string) {
  const q = query.trim().toLowerCase()
  const match = (e: EntryPoint) => !q || e.label.toLowerCase().includes(q) || nameOf(e.node).toLowerCase().includes(q)
  return ENTRY_GROUPS.map((g) => ({ ...g, entries: entries.filter((e) => e.kind === g.kind && match(e)) })).filter(
    (g) => g.entries.length > 0,
  )
}
