import { describe, expect, it } from "vitest"

import type { Interaction } from "@/shared/bindings/Interaction"
import type { SystemNode } from "@/shared/bindings/SystemNode"

import { featureLayers, featureMatrix } from "./boundaries"

const node = (id: string, feature: string, layer: SystemNode["layer"] = "application"): SystemNode => ({
  id,
  kind: "component",
  name: id,
  role: "service",
  layer,
  package: "api",
  feature,
  symbolIds: [],
  file: null,
  line: null,
  description: null,
  hidden: false,
  merged: [],
})

const call = (from: string, to: string, count = 1, kind: Interaction["kind"] = "call"): Interaction => ({
  id: `${from}->${to}:${kind}`,
  from,
  to,
  kind,
  members: [],
  sites: [],
  count,
})

describe("boundaries", () => {
  const model = {
    nodes: [node("A", "claims", "interface"), node("B", "claims"), node("C", "billing", "data"), node("D", "mail")],
    interactions: [call("A", "B"), call("B", "C", 3), call("C", "D", 2, "emits"), call("D", "C", 1, "http")],
    entryPoints: [],
  }

  it("counts components per feature and layer", () => {
    const rows = featureLayers(model)
    expect(rows[0]).toMatchObject({ label: "claims", total: 2, byLayer: { interface: 1, application: 1 } })
  })

  it("builds a dependency matrix between features, ignoring events and internal calls", () => {
    const m = featureMatrix(model)
    expect(m.features.map((f) => f.label)).toEqual(["billing", "claims", "mail"])
    expect(m.cells).toEqual([
      [0, 0, 0],
      [3, 0, 0],
      [1, 0, 0],
    ])
    expect(m.max).toBe(3)
  })
})
