import { describe, expect, it } from "vitest"

import type { DiagramView } from "../model/view"
import { flattenLayout, toElkGraph } from "./elk-graph"

const symbol = (id: string, parent: string): DiagramView["nodes"][number] => ({
  type: "symbol",
  id,
  parent,
  symbol: {
    id, kind: "class", name: id, file: "a.ts", range: { startLine: 1, endLine: 1 }, package: "a",
    exported: true, isAbstract: false, typeParams: null, signature: "", members: [],
  },
})

describe("toElkGraph", () => {
  const view: DiagramView = {
    nodes: [
      { type: "group", id: "pkg:a", level: "package", label: "a", expanded: true, symbolCount: 2 },
      { type: "group", id: "pkg:b", level: "package", label: "b", expanded: false, symbolCount: 3 },
      symbol("A", "pkg:a"),
      symbol("B", "pkg:a"),
    ],
    edges: [
      { id: "ab", source: "A", target: "B", kind: "extends", count: 1 },
      { id: "a-b", source: "A", target: "pkg:b", kind: "aggregate", count: 2 },
    ],
  }

  it("nests expanded groups and keeps collapsed ones as sized leaves", () => {
    const { graph } = toElkGraph(view)
    expect(graph.children?.map((c) => c.id)).toEqual(["pkg:a", "pkg:b"])
    expect(graph.children?.[0].children?.map((c) => c.id)).toEqual(["A", "B"])
    expect(graph.children?.[1].width).toBeGreaterThan(0)
  })

  it("routes sibling edges in their container and leaves cross-container edges to the canvas", () => {
    const { graph, routed } = toElkGraph(view)
    expect(graph.children?.[0].edges?.map((e) => e.id)).toEqual(["ab"])
    expect([...routed]).toEqual(["ab"])
  })

  it("flattens nested positions and makes routes absolute", () => {
    const layout = flattenLayout({
      id: "root",
      children: [
        {
          id: "p", x: 100, y: 50, width: 10, height: 10,
          children: [{ id: "c", x: 1, y: 2, width: 3, height: 4 }],
          edges: [{ id: "e", sources: [], targets: [], sections: [{ id: "s", startPoint: { x: 1, y: 1 }, endPoint: { x: 5, y: 1 } }] }],
        },
      ],
    })
    expect(layout.nodes.get("c")).toEqual({ id: "c", x: 1, y: 2, width: 3, height: 4 })
    expect(layout.routes.get("e")).toEqual([{ x: 101, y: 51 }, { x: 105, y: 51 }])
  })
})
