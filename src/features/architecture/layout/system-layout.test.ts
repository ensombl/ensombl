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
  it("nests packages and returns edge routes in root coordinates", async () => {
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
    const layout = flattenLayout(await new ELK().layout(toSystemElk(view)), true)
    const abs = (id: string, parent?: string) => {
      const box = layout.nodes.get(id)!
      const p = parent ? layout.nodes.get(parent)! : { x: 0, y: 0 }
      return { x: p.x + box.x, y: p.y + box.y, right: p.x + box.x + box.width }
    }
    const page = abs("Page", "pkg:web")
    const svc = abs("Svc", "pkg:api")
    const route = layout.routes.get("e1")!
    // Starts at the page's right edge and ends at the service's left edge, in absolute terms.
    expect(Math.abs(route[0].x - page.right)).toBeLessThan(2)
    expect(Math.abs(route[route.length - 1].x - svc.x)).toBeLessThan(2)
    // The external sits outside both boxes, to the right.
    expect(abs("ext:Db").x).toBeGreaterThan(svc.right)
  })
})
