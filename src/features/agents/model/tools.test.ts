import { describe, expect, it } from "vitest"

import { diffLines, fileEdits, questionsOf, summarizeTool, todosOf } from "./tools"

describe("summarizeTool", () => {
  it("shows file tools relative to the worktree", () => {
    expect(summarizeTool("Edit", { file_path: "/repo/src/a.ts" }, "/repo")).toEqual({ title: "Edit", detail: "src/a.ts", file: "src/a.ts" })
  })

  it("prefers a command's description and names MCP tools by server", () => {
    expect(summarizeTool("Bash", { command: "pnpm test\n--watch", description: "Run tests" }, "/repo").detail).toBe("Run tests")
    expect(summarizeTool("Bash", { command: "pnpm test\n--watch" }, "/repo").detail).toBe("pnpm test")
    expect(summarizeTool("mcp__github__create_issue", {}, "/repo")).toEqual({ title: "create issue", detail: "github" })
  })
})

describe("diffLines", () => {
  it("keeps common lines and marks changes", () => {
    expect(diffLines("a\nb\nc", "a\nB\nc")).toEqual([
      { op: " ", text: "a" },
      { op: "-", text: "b" },
      { op: "+", text: "B" },
      { op: " ", text: "c" },
    ])
  })
})

describe("tool inputs", () => {
  it("reads edits, todos and questions defensively", () => {
    expect(fileEdits("MultiEdit", { edits: [{ old_string: "x", new_string: "y" }] })).toEqual([{ before: "x", after: "y" }])
    expect(fileEdits("Bash", { command: "ls" })).toEqual([])
    expect(todosOf({ todos: [{ content: "a", status: "pending" }, { nope: 1 }] })).toHaveLength(1)
    expect(questionsOf({ questions: [{ question: "Which?", options: [{ label: "A" }] }, { question: 3 }] })).toHaveLength(1)
    expect(todosOf(null)).toEqual([])
  })
})
