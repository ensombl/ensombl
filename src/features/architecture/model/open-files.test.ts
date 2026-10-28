import { describe, expect, it } from "vitest"

import { closeTab, openTab } from "./open-files"

describe("open file tabs", () => {
  it("opens a file once, after the active tab", () => {
    expect(openTab(["a", "b"], "a", "c")).toEqual(["a", "c", "b"])
    expect(openTab(["a", "b"], null, "c")).toEqual(["a", "b", "c"])
    const tabs = ["a", "b"]
    expect(openTab(tabs, "a", "b")).toBe(tabs)
  })

  it("moves to a neighbour when the active tab closes", () => {
    expect(closeTab(["a", "b", "c"], "b", "b")).toEqual({ tabs: ["a", "c"], active: "c" })
    expect(closeTab(["a", "b"], "b", "b")).toEqual({ tabs: ["a"], active: "a" })
    expect(closeTab(["a"], "a", "a")).toEqual({ tabs: [], active: null })
    expect(closeTab(["a", "b"], "a", "b")).toEqual({ tabs: ["a"], active: "a" })
  })
})
