import { describe, expect, it } from "vitest"

import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { SymbolNode } from "@/shared/bindings/SymbolNode"

import { applyDelta } from "./apply-delta"

const sym = (id: string, signature = ""): SymbolNode => ({
  id, kind: "class", name: id, file: "f.ts", range: { startLine: 1, endLine: 1 }, package: "p",
  exported: true, isAbstract: false, typeParams: null, signature, members: [],
})

describe("applyDelta", () => {
  it("upserts, removes and keeps symbols sorted; patches edges", () => {
    const model: ArchModel = {
      root: "/r",
      symbols: [sym("a"), sym("b"), sym("c")],
      edges: [
        { from: "a", to: "b", kind: "dependency" },
        { from: "c", to: "a", kind: "extends" },
      ],
    }
    const next = applyDelta(model, {
      upserted: [sym("b", "changed"), sym("aa")],
      removed: ["c"],
      edgesAdded: [{ from: "aa", to: "a", kind: "association" }],
      edgesRemoved: [{ from: "a", to: "b", kind: "dependency" }],
    })
    expect(next.symbols.map((s) => s.id)).toEqual(["a", "aa", "b"])
    expect(next.symbols[2].signature).toBe("changed")
    expect(next.edges).toEqual([{ from: "aa", to: "a", kind: "association" }])
    expect(model.symbols).toHaveLength(3)
  })
})
