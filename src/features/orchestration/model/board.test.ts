import { describe, expect, it } from "vitest"

import type { Task } from "@/shared/bindings/Task"

import { byColumn } from "./board"

const task = (id: string, status: Task["status"], updatedAt: number): Task => ({
  id,
  projectId: "p",
  title: id,
  prompt: "",
  agent: "claude",
  baseBranch: "main",
  branch: `ensombl/${id}`,
  worktree: null,
  status,
  dependsOn: [],
  permission: "acceptEdits",
  sessionId: null,
  nextPrompt: null,
  summary: null,
  error: null,
  createdAt: 0,
  updatedAt,
})

describe("byColumn", () => {
  it("groups blocked tasks with running ones and failures with cancellations, newest first", () => {
    const columns = byColumn([task("a", "running", 1), task("b", "needsInput", 2), task("c", "failed", 1), task("d", "cancelled", 3)])
    expect(columns.running.map((t) => t.id)).toEqual(["b", "a"])
    expect(columns.stopped.map((t) => t.id)).toEqual(["d", "c"])
    expect(columns.backlog).toEqual([])
  })
})
