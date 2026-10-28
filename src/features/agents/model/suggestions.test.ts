import { describe, expect, it } from "vitest"

import type { ArchModel } from "@/shared/bindings/ArchModel"

import { matchSuggestions, mentionAt, taggables } from "./suggestions"

const model = {
  root: "/r",
  edges: [],
  symbols: [
    { id: "a.ts#UserService", name: "UserService", file: "src/a.ts", kind: "class", members: [{ name: "save", kind: "method", signature: "save()" }] },
    { id: "b.ts#User", name: "User", file: "src/b.ts", kind: "interface", members: [] },
    { id: "c.ts#AdminUser", name: "AdminUser", file: "src/c.ts", kind: "class", members: [] },
  ],
} as unknown as ArchModel

describe("suggestions", () => {
  it("ranks prefix matches before substring matches", () => {
    const labels = matchSuggestions(taggables(model), "user").map((s) => s.tag.label)
    expect(labels.slice(0, 3)).toEqual(["User", "UserService", "AdminUser"])
  })

  it("only offers members once a dot is typed", () => {
    expect(matchSuggestions(taggables(model), "save")).toHaveLength(0)
    expect(matchSuggestions(taggables(model), "UserService.s")[0].tag.label).toBe("UserService.save")
  })

  it("finds the mention being typed at the caret", () => {
    expect(mentionAt("fix @Use", 8)).toEqual({ start: 4, query: "Use" })
    expect(mentionAt("email@x", 7)).toBeNull()
    expect(mentionAt("done @A then", 12)).toBeNull()
  })
})
