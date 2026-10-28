import { describe, expect, it } from "vitest"

import type { AnchoredComment } from "@/shared/bindings/AnchoredComment"

import { reviewPrompt } from "./review-prompt"

const comment = (id: string, line: number | null, body: string, resolved = false): AnchoredComment => ({
  comment: { id, file: "src/a.ts", line: 3, body, anchor: "return x", resolved, createdAt: 0 },
  currentLine: line,
})

describe("reviewPrompt", () => {
  it("lists open comments with locations and code", () => {
    const out = reviewPrompt([comment("1", 5, "handle null"), comment("2", 3, "done", true)])
    expect(out).toBe("Please address these review comments:\n\n1. @src/a.ts:5\n   > return x\n   handle null")
  })

  it("flags comments whose code moved away", () => {
    expect(reviewPrompt([comment("1", null, "x")])).toContain("@src/a.ts (code has changed since)")
  })

  it("is empty with nothing open", () => {
    expect(reviewPrompt([comment("1", 1, "x", true)])).toBe("")
  })
})
