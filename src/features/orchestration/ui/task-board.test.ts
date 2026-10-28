import { describe, expect, it } from "vitest"

import { trouble } from "./task-board"

describe("trouble", () => {
  it("prefers conflicts over overlaps and stays quiet when clean", () => {
    expect(trouble(undefined)).toBeNull()
    expect(trouble({ taskId: "a", behind: 3, conflicts: [], overlaps: [] })).toBeNull()
    expect(trouble({ taskId: "a", behind: 0, conflicts: [], overlaps: [{ taskId: "b", files: ["x"] }] })).toBe("Touches the same files as 1 other task")
    expect(trouble({ taskId: "a", behind: 0, conflicts: ["x", "y"], overlaps: [{ taskId: "b", files: ["x"] }] })).toBe("Conflicts with its base in 2 files")
  })
})
