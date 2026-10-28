import { describe, expect, it } from "vitest"

import { layoutCache } from "./layout-cache"

describe("layoutCache", () => {
  it("keeps the most recently used entries", () => {
    const cache = layoutCache<number>(2)
    cache.set("a", 1)
    cache.set("b", 2)
    expect(cache.get("a")).toBe(1)
    cache.set("c", 3)
    expect(cache.get("b")).toBeUndefined()
    expect(cache.get("a")).toBe(1)
    expect(cache.get("c")).toBe(3)
  })
})
