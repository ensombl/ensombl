import { describe, expect, it } from "vitest"

import type { Interaction } from "@/shared/bindings/Interaction"
import type { SystemModel } from "@/shared/bindings/SystemModel"
import type { SystemNode } from "@/shared/bindings/SystemNode"

import { buildComponentsView, defaultFilters, featureId, membersLabel, OVERVIEW_THRESHOLD } from "./system-view"

const node = (id: string, feature: string, extra: Partial<SystemNode> = {}): SystemNode => ({
  id,
  kind: "component",
  name: id,
  role: "service",
  layer: "application",
  package: "api",
  feature,
  symbolIds: [id],
  file: `${feature}/${id}.ts`,
  line: 1,
  description: null,
  hidden: false,
  merged: [],
  ...extra,
})

const link = (from: string, to: string, kind: Interaction["kind"] = "call", members = ["run"]): Interaction => ({
  id: `${from}->${to}:${kind}`,
  from,
  to,
  kind,
  members,
  sites: [],
  count: members.length,
})

const model = (nodes: SystemNode[], interactions: Interaction[]): SystemModel => ({ nodes, interactions, entryPoints: [] })

describe("buildComponentsView", () => {
  const small = model(
    [
      node("ClaimsController", "claims", { role: "controller", layer: "interface" }),
      node("ClaimsService", "claims"),
      node("Billing", "billing"),
      node("Helper", "claims", { role: "other" }),
      node("ext:Database", "external", { kind: "external", role: "external", layer: "external" }),
      node("channel:paid", "events", { kind: "channel", role: "channel" }),
      node("Mailer", "mail"),
    ],
    [
      link("ClaimsController", "ClaimsService"),
      link("ClaimsService", "Billing", "call", ["charge"]),
      link("ClaimsService", "ext:Database", "call", ["Claim.save"]),
      link("Billing", "channel:paid", "emits", ["paid"]),
      link("channel:paid", "Mailer", "handles", ["send"]),
      link("ClaimsService", "Helper"),
    ],
  )

  it("shows small systems whole, without unclassified code", () => {
    const view = buildComponentsView(small, null, defaultFilters())
    expect(view.overview).toBe(false)
    expect(view.nodes.map((n) => n.id)).not.toContain("Helper")
    expect(view.edges).toHaveLength(5)
  })

  it("focuses a feature with its neighbors as context, following channels", () => {
    const view = buildComponentsView(small, featureId({ package: "api", feature: "billing" }), defaultFilters())
    const byId = Object.fromEntries(view.nodes.map((n) => [n.id, n]))
    expect(Object.keys(byId).sort()).toEqual(["Billing", "ClaimsService", "Mailer", "channel:paid"])
    expect(byId.ClaimsService).toMatchObject({ context: true })
    expect(byId.Billing).toMatchObject({ context: false })
  })

  it("opens big systems as features with summed edges and events passed through", () => {
    const many = Array.from({ length: OVERVIEW_THRESHOLD }, (_, i) => node(`S${i}`, "claims"))
    const view = buildComponentsView(model([...small.nodes, ...many], small.interactions), null, defaultFilters())
    expect(view.overview).toBe(true)
    const ids = view.edges.map((e) => `${e.source}->${e.target}`).sort()
    expect(ids).toEqual([
      "feature:api/billing->feature:api/mail",
      "feature:api/claims->ext:Database",
      "feature:api/claims->feature:api/billing",
    ])
    expect(view.nodes.find((n) => n.id === "feature:api/claims")).toMatchObject({ count: 62 })
  })

  it("labels edges compactly", () => {
    expect(membersLabel(["a", "b", "c", "d"])).toBe("a, b +2")
    expect(membersLabel([])).toBe("")
  })
})
