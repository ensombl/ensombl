import { beforeEach, describe, expect, it } from "vitest"

import { useChangesStore } from "./changes-store"

describe("changes store", () => {
  beforeEach(() => useChangesStore.setState({ byWorktree: {} }))

  it("marks, acknowledges and clears per worktree", () => {
    const { mark, ack, clear } = useChangesStore.getState()
    mark("wt", ["a", "b"])
    mark("other", ["a"])
    expect(Object.keys(useChangesStore.getState().byWorktree.wt)).toEqual(["a", "b"])

    ack("wt", ["a"])
    expect(Object.keys(useChangesStore.getState().byWorktree.wt)).toEqual(["b"])
    expect(Object.keys(useChangesStore.getState().byWorktree.other)).toEqual(["a"])

    clear("wt")
    expect(useChangesStore.getState().byWorktree.wt).toEqual({})
  })
})
