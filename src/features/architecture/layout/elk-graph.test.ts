import { describe, expect, it } from "vitest"

import type { DiagramView } from "../model/view"
import { flattenLayout, toElkGraph } from "./elk-graph"

describe("toElkGraph", () => {
  it("nests expanded groups and keeps collapsed ones as sized leaves", () => {
    const view: DiagramView = {
      nodes: [
        { type: "group", id: "pkg:a", level: "package", label: "a", expanded: true, symbolCount: 1 },
        { type: "group", id: "pkg:b", level: "package", label: "b", expanded: false, symbolCount: 3 },
      ],
      edges: [{ id: "e", source: "pkg:a", target: "pkg:b", kind: "aggregate", count: 2 }],
    }
    view.nodes.push({
      type: "symbol",
      id: "a.ts#A",
      parent: "pkg:a",
      open: false,
      symbol: {
        id: "a.ts#A", kind: "class", name: "A", file: "a.ts", range: { startLine: 1, endLine: 1 }, package: "a",
        exported: true, isAbstract: false, typeParams: null, signature: "class A", members: [],
      },
    })
    const graph = toElkGraph(view)
    expect(graph.children?.map((c) => c.id)).toEqual(["pkg:a", "pkg:b"])
    expect(graph.children?.[0].children?.[0].id).toBe("a.ts#A")
    expect(graph.children?.[1].width).toBeGreaterThan(0)
    expect(graph.edges?.[0]).toMatchObject({ sources: ["pkg:a"], targets: ["pkg:b"] })
  })

  it("flattens nested positions", () => {
    const flat = flattenLayout({ id: "root", children: [{ id: "p", x: 5, y: 6, width: 10, height: 10, children: [{ id: "c", x: 1, y: 2, width: 3, height: 4 }] }] })
    expect(flat.get("c")).toEqual({ id: "c", x: 1, y: 2, width: 3, height: 4 })
    expect(flat.get("p")?.x).toBe(5)
  })
})
