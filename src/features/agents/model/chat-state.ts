import type { ChatEvent } from "@/shared/bindings/ChatEvent"
import type { ContentBlock } from "@/shared/bindings/ContentBlock"
import type { NoticeLevel } from "@/shared/bindings/NoticeLevel"
import type { PermissionMode } from "@/shared/bindings/PermissionMode"

/**
 * Folds a chat's event stream into a timeline. Pure and immutable: each
 * update copies only the path to the changed item, so unchanged items keep
 * their identity and memoized rows don't re-render while text streams in.
 */

export interface PermissionRequest {
  requestId: string
  toolName: string
  input: unknown
  description: string | null
  /** `undefined` while waiting for the user. */
  allowed?: boolean
}

export type ChatItem =
  | { kind: "user"; id: string; text: string }
  | { kind: "text"; id: string; text: string }
  | { kind: "thinking"; id: string; text: string }
  | {
      kind: "tool"
      id: string
      name: string
      input: unknown
      result?: { content: string; isError: boolean }
      permission?: PermissionRequest
      /** Subagent output (Task tool). */
      children: ChatItem[]
    }
  | { kind: "permission"; id: string; request: PermissionRequest }
  | {
      kind: "turn"
      id: string
      isError: boolean
      result: string | null
      durationMs: number | null
      costUsd: number | null
    }
  | { kind: "notice"; id: string; level: NoticeLevel; text: string }

export interface ChatState {
  items: ChatItem[]
  /** Item id to its path through `items` and `children`. */
  paths: Record<string, number[]>
  /** Messages whose blocks were streamed, so their final copy only fills in tool input. */
  streamed: Record<string, true>
  busy: boolean
  running: boolean
  sessionId: string | null
  model: string | null
  mode: PermissionMode
  costUsd: number | null
  seq: number
}

export const emptyChat = (): ChatState => ({
  items: [],
  paths: {},
  streamed: {},
  busy: false,
  running: true,
  sessionId: null,
  model: null,
  mode: "default",
  costUsd: null,
  seq: 0,
})

function updateAt(items: ChatItem[], path: number[], fn: (item: ChatItem) => ChatItem): ChatItem[] {
  const [i, ...rest] = path
  const next = items.slice()
  const item = items[i]
  if (rest.length === 0) next[i] = fn(item)
  else if (item.kind === "tool") next[i] = { ...item, children: updateAt(item.children, rest, fn) }
  return next
}

function update(state: ChatState, id: string, fn: (item: ChatItem) => ChatItem): ChatState {
  const path = state.paths[id]
  return path ? { ...state, items: updateAt(state.items, path, fn) } : state
}

/** Appends at the top level, or inside the subagent's tool card when `parent` is known. */
function append(state: ChatState, item: ChatItem, parent: string | null = null): ChatState {
  const parentPath = parent ? state.paths[parent] : undefined
  if (!parentPath) {
    return { ...state, items: [...state.items, item], paths: { ...state.paths, [item.id]: [state.items.length] } }
  }
  let index = 0
  const items = updateAt(state.items, parentPath, (p) => {
    if (p.kind !== "tool") return p
    index = p.children.length
    return { ...p, children: [...p.children, item] }
  })
  return { ...state, items, paths: { ...state.paths, [item.id]: [...parentPath, index] } }
}

function blockItem(id: string, block: ContentBlock): ChatItem {
  if (block.type === "toolUse") return { kind: "tool", id: block.id, name: block.name, input: block.input, children: [] }
  return { kind: block.type, id, text: block.text }
}

const withText = (item: ChatItem, text: string): ChatItem =>
  item.kind === "text" || item.kind === "thinking" ? { ...item, text: item.text + text } : item

export function reduceChat(state: ChatState, event: ChatEvent): ChatState {
  const seq = state.seq + 1
  const s = { ...state, seq }
  switch (event.type) {
    case "session":
      return { ...s, sessionId: event.sessionId, model: event.model ?? s.model, mode: event.permissionMode ?? s.mode }
    case "user":
      return { ...append(s, { kind: "user", id: `user:${event.id}`, text: event.text }), busy: true }
    case "blockStart": {
      const id = event.block.type === "toolUse" ? event.block.id : `${event.messageId}:${event.index}`
      if (s.paths[id]) return s
      const next = append(s, blockItem(id, event.block), event.parent)
      return { ...next, streamed: { ...next.streamed, [event.messageId]: true } }
    }
    case "blockDelta":
      return update(s, `${event.messageId}:${event.index}`, (item) => withText(item, event.text))
    case "assistant": {
      let next = s
      event.blocks.forEach((block, n) => {
        if (block.type === "toolUse") {
          next = next.paths[block.id]
            ? update(next, block.id, (item) => (item.kind === "tool" ? { ...item, input: block.input } : item))
            : append(next, blockItem(block.id, block), event.parent)
        } else if (!next.streamed[event.messageId]) {
          next = append(next, blockItem(`${event.messageId}:a${seq}:${n}`, block), event.parent)
        }
      })
      return next
    }
    case "toolResult":
      return update(s, event.toolUseId, (item) =>
        item.kind === "tool" ? { ...item, result: { content: event.content, isError: event.isError } } : item,
      )
    case "permission": {
      const request: PermissionRequest = {
        requestId: event.requestId,
        toolName: event.toolName,
        input: event.input,
        description: event.description,
      }
      const toolId = event.toolUseId
      if (toolId && s.paths[toolId]) {
        const next = update(s, toolId, (item) => (item.kind === "tool" ? { ...item, permission: request } : item))
        return { ...next, paths: { ...next.paths, [`perm:${event.requestId}`]: next.paths[toolId] } }
      }
      return append(s, { kind: "permission", id: `perm:${event.requestId}`, request })
    }
    case "permissionResolved":
      return update(s, `perm:${event.requestId}`, (item) => {
        if (item.kind === "permission") return { ...item, request: { ...item.request, allowed: event.allowed } }
        if (item.kind === "tool" && item.permission) return { ...item, permission: { ...item.permission, allowed: event.allowed } }
        return item
      })
    case "turnComplete": {
      const next = append(s, {
        kind: "turn",
        id: `turn:${seq}`,
        isError: event.isError,
        result: event.result,
        durationMs: event.durationMs,
        costUsd: event.costUsd,
      })
      return {
        ...next,
        busy: false,
        costUsd: event.costUsd ?? next.costUsd,
      }
    }
    case "modeChanged":
      return { ...s, mode: event.mode }
    case "modelChanged":
      return { ...s, model: event.model }
    case "notice":
      return append(s, { kind: "notice", id: `notice:${seq}`, level: event.level, text: event.text })
    case "exited":
      return { ...s, running: false, busy: false }
  }
}

export const reduceAll = (events: ChatEvent[], state = emptyChat()) => events.reduce(reduceChat, state)

/** Permission requests still waiting for an answer, oldest first. */
export function pendingPermissions(items: ChatItem[]): PermissionRequest[] {
  const out: PermissionRequest[] = []
  for (const item of items) {
    if (item.kind === "permission" && item.request.allowed === undefined) out.push(item.request)
    if (item.kind === "tool") {
      if (item.permission && item.permission.allowed === undefined) out.push(item.permission)
      out.push(...pendingPermissions(item.children))
    }
  }
  return out
}
