import { describe, expect, it } from "vitest"

import { suggestBranchName } from "./branch-names"

describe("suggestBranchName", () => {
  it("avoids taken names", () => {
    const name = suggestBranchName(["graysby"], () => 0)
    expect(name).toBe("grouper")
  })

  it("falls back to a numbered name when every fish is taken", () => {
    const all = Array.from({ length: 30 }, (_, i) => suggestBranchName([], () => i / 30))
    expect(suggestBranchName(all)).toBe("graysby-2")
  })
})
