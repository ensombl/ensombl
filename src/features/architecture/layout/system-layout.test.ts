import ELK from "elkjs/lib/elk.bundled.js"
import { describe, expect, it } from "vitest"

import type { SystemNode } from "@/shared/bindings/SystemNode"

import type { ComponentsView } from "../model/system-view"
import { flattenLayout, toSystemElk } from "./system-layout"

const node = (id: string, pkg: string, extra: Partial<SystemNode> = {}): SystemNode => ({
  id,
  kind: "component",
  name: id,
  role: "service",
  layer: "application",
  package: pkg,
  feature: "f",
  symbolIds: [],
  file: null,
  line: null,
  description: null,
  hidden: false,
  merged: [],
  ...extra,
})

describe("toSystemElk", () => {
  it("lays out each package box on its own and keeps externals outside", async () => {
    const view: ComponentsView = {
      overview: false,
      grouped: true,
      nodes: [
        { type: "package", id: "pkg:web", label: "web", count: 1 },
        { type: "package", id: "pkg:api", label: "api", count: 1 },
        { type: "component", id: "Page", parent: "pkg:web", node: node("Page", "web"), entryCount: 0, context: false },
        { type: "component", id: "Svc", parent: "pkg:api", node: node("Svc", "api"), entryCount: 0, context: false },
        { type: "component", id: "ext:Db", node: node("ext:Db", "", { kind: "external" }), entryCount: 0, context: false },
      ],
      edges: [
        { id: "e1", source: "Page", target: "Svc", kind: "http", members: [], count: 1 },
        { id: "e2", source: "Svc", target: "ext:Db", kind: "call", members: [], count: 1 },
      ],
    }
    view.nodes.push({ type: "component", id: "Svc2", parent: "pkg:api", node: node("Svc2", "api"), entryCount: 0, context: false })
    view.edges.push({ id: "e3", source: "Svc", target: "Svc2", kind: "call", members: [], count: 1 })
    const layout = flattenLayout(await new ELK().layout(toSystemElk(view)))
    const abs = (id: string, parent?: string) => {
      const box = layout.nodes.get(id)!
      const p = parent ? layout.nodes.get(parent)! : { x: 0, y: 0 }
      return { x: p.x + box.x, y: p.y + box.y, right: p.x + box.x + box.width }
    }
    const page = abs("Page", "pkg:web")
    const svc = abs("Svc", "pkg:api")
    const svc2 = abs("Svc2", "pkg:api")
    // Calls flow right: web's box before api's.
    expect(page.right).toBeLessThan(svc.x)
    // Edges inside a box are routed, in absolute coordinates.
    const route = layout.routes.get("e3")!
    expect(Math.abs(route[0].x - svc.right)).toBeLessThan(2)
    expect(Math.abs(route[route.length - 1].x - svc2.x)).toBeLessThan(2)
    // Edges across boxes are left to the canvas.
    expect(layout.routes.has("e1")).toBe(false)
    // The external sits outside both boxes, to the right.
    expect(abs("ext:Db").x).toBeGreaterThan(Math.max(svc.right, svc2.right))
  })
})
