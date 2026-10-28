import type { RunEvent } from "@/shared/bindings/RunEvent"

/** One line of a run's activity timeline. Deliberately a log, not a chat. */
export type ActivityItem =
  | { kind: "text"; text: string }
  | { kind: "tool"; name: string; target: string }
  | { kind: "error"; text: string }
  | { kind: "notice"; text: string }
  | { kind: "permission"; tool: string; allowed?: boolean }
  | { kind: "result"; ok: boolean; text: string | null; costUsd: number | null; durationMs: number | null }
  | { kind: "exit"; code: number | null }

/** The most telling argument of a tool call: its file, command, pattern or URL. */
export function toolTarget(input: unknown): string {
  if (!input || typeof input !== "object") return ""
  const record = input as Record<string, unknown>
  for (const key of ["file_path", "path", "command", "pattern", "url", "query", "description"]) {
    const value = record[key]
    if (typeof value === "string" && value) return value.length > 120 ? `${value.slice(0, 117)}…` : value
  }
  return ""
}

/**
 * Folds a run's events into a timeline. Complete assistant messages are used,
 * not streaming deltas; tool results only appear when they failed.
 */
export function toActivity(events: RunEvent[]): ActivityItem[] {
  const items: ActivityItem[] = []
  const permissions = new Map<string, Extract<ActivityItem, { kind: "permission" }>>()
  for (const event of events) {
    switch (event.type) {
      case "assistant":
        for (const block of event.blocks) {
          if (block.type === "text" && block.text.trim()) items.push({ kind: "text", text: block.text.trim() })
          if (block.type === "toolUse") items.push({ kind: "tool", name: block.name, target: toolTarget(block.input) })
        }
        break
      case "toolResult":
        if (event.isError) items.push({ kind: "error", text: event.content.slice(0, 500) })
        break
      case "notice":
        items.push({ kind: event.level === "error" ? "error" : "notice", text: event.text })
        break
      case "permission": {
        const item = { kind: "permission" as const, tool: event.toolName }
        permissions.set(event.requestId, item)
        items.push(item)
        break
      }
      case "permissionResolved": {
        const item = permissions.get(event.requestId)
        if (item) item.allowed = event.allowed
        break
      }
      case "turnComplete":
        items.push({ kind: "result", ok: !event.isError, text: event.result, costUsd: event.costUsd, durationMs: event.durationMs })
        break
      case "exited":
        items.push({ kind: "exit", code: event.code })
        break
    }
  }
  return items
}
