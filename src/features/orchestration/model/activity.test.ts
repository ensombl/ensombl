import { describe, expect, it } from "vitest"

import type { RunEvent } from "@/shared/bindings/RunEvent"

import { toActivity, toolTarget } from "./activity"

describe("toActivity", () => {
  it("keeps text, tool calls, failures and the result; skips streaming noise", () => {
    const events: RunEvent[] = [
      { type: "session", sessionId: "s", model: null, permissionMode: null },
      { type: "blockDelta", messageId: "m", index: 0, text: "partial" },
      {
        type: "assistant",
        messageId: "m",
        parent: null,
        blocks: [
          { type: "text", text: "Adding the page." },
          { type: "toolUse", id: "t1", name: "Write", input: { file_path: "src/login.tsx", content: "…" } },
        ],
      },
      { type: "toolResult", toolUseId: "t1", content: "ok", isError: false },
      { type: "toolResult", toolUseId: "t2", content: "boom", isError: true },
      { type: "permission", requestId: "r", toolName: "Bash", toolUseId: null, input: {}, description: null },
      { type: "permissionResolved", requestId: "r", allowed: false },
      { type: "turnComplete", isError: false, result: "Done", durationMs: 1200, costUsd: 0.1, inputTokens: null, outputTokens: null },
    ]
    expect(toActivity(events)).toEqual([
      { kind: "text", text: "Adding the page." },
      { kind: "tool", name: "Write", target: "src/login.tsx" },
      { kind: "error", text: "boom" },
      { kind: "permission", tool: "Bash", allowed: false },
      { kind: "result", ok: true, text: "Done", costUsd: 0.1, durationMs: 1200 },
    ])
  })

  it("picks the most telling tool argument", () => {
    expect(toolTarget({ command: "pnpm test" })).toBe("pnpm test")
    expect(toolTarget({ other: 1 })).toBe("")
    expect(toolTarget(null)).toBe("")
  })
})
