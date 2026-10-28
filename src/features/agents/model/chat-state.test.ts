import { describe, expect, it } from "vitest"

import type { ChatEvent } from "@/shared/bindings/ChatEvent"
import type { ContentBlock } from "@/shared/bindings/ContentBlock"

import { pendingPermissions, reduceAll } from "./chat-state"

const start = (messageId: string, index: number, block: ContentBlock, parent: string | null = null): ChatEvent => ({
  type: "blockStart",
  messageId,
  parent,
  index,
  block,
})

describe("reduceChat", () => {
  it("streams text and ignores the duplicate final copy", () => {
    const state = reduceAll([
      { type: "user", id: "u1", text: "hi" },
      start("m1", 0, { type: "text", text: "" }),
      { type: "blockDelta", messageId: "m1", index: 0, text: "Hel" },
      { type: "blockDelta", messageId: "m1", index: 0, text: "lo" },
      { type: "assistant", messageId: "m1", parent: null, blocks: [{ type: "text", text: "Hello" }] },
    ])
    expect(state.busy).toBe(true)
    expect(state.items.map((i) => i.kind)).toEqual(["user", "text"])
    expect(state.items[1]).toMatchObject({ text: "Hello" })
  })

  it("fills tool input from the final message and attaches results and permissions", () => {
    const state = reduceAll([
      start("m1", 0, { type: "toolUse", id: "t1", name: "Write", input: {} }),
      { type: "assistant", messageId: "m1", parent: null, blocks: [{ type: "toolUse", id: "t1", name: "Write", input: { file_path: "a.ts" } }] },
      { type: "permission", requestId: "r1", toolName: "Write", toolUseId: "t1", input: { file_path: "a.ts" }, description: null },
    ])
    expect(pendingPermissions(state.items)).toHaveLength(1)

    const done = reduceAll(
      [
        { type: "permissionResolved", requestId: "r1", allowed: true },
        { type: "toolResult", toolUseId: "t1", content: "ok", isError: false },
      ],
      state,
    )
    expect(pendingPermissions(done.items)).toHaveLength(0)
    expect(done.items[0]).toMatchObject({ kind: "tool", input: { file_path: "a.ts" }, result: { content: "ok" }, permission: { allowed: true } })
  })

  it("nests subagent output under its task and keeps unrelated items stable", () => {
    const state = reduceAll([
      { type: "user", id: "u1", text: "go" },
      start("m1", 0, { type: "toolUse", id: "task", name: "Task", input: {} }),
      { type: "assistant", messageId: "s1", parent: "task", blocks: [{ type: "text", text: "sub" }, { type: "toolUse", id: "t2", name: "Read", input: {} }] },
    ])
    const before = state.items[0]
    const after = reduceAll([{ type: "toolResult", toolUseId: "t2", content: "file", isError: false }], state)
    const task = after.items[1]
    expect(task.kind === "tool" && task.children.map((c) => c.kind)).toEqual(["text", "tool"])
    expect(task.kind === "tool" && task.children[1]).toMatchObject({ result: { content: "file" } })
    expect(after.items[0]).toBe(before)
  })

  it("tracks session, cost and exit", () => {
    const state = reduceAll([
      { type: "session", sessionId: "s", model: "opus", permissionMode: "plan" },
      { type: "user", id: "u1", text: "hi" },
      { type: "turnComplete", isError: false, result: "ok", durationMs: 10, costUsd: 0.2, inputTokens: 1200, outputTokens: 3 },
      { type: "modeChanged", mode: "acceptEdits" },
      { type: "exited", code: 0 },
    ])
    expect(state).toMatchObject({ sessionId: "s", model: "opus", mode: "acceptEdits", costUsd: 0.2, busy: false, running: false })
  })
})
