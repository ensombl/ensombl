import { describe, expect, it } from "vitest"

import type { EntryPoint } from "@/shared/bindings/EntryPoint"
import type { FlowStep } from "@/shared/bindings/FlowStep"

import { groupEntries, toSequence } from "./sequence"

const step = (from: string, to: string, label: string, depth = 0): FlowStep => ({
  depth,
  from,
  to,
  label,
  kind: "call",
  isAsync: false,
  site: { file: "a.ts", line: 1 },
})

describe("toSequence", () => {
  it("orders lifelines by first appearance from the entry", () => {
    const seq = toSequence("C", [step("C", "S", "submit"), step("S", "S", "validate", 1), step("S", "R", "save", 1), step("C", "R", "audit")])
    expect(seq.participants).toEqual(["C", "S", "R"])
    expect(seq.rows.map((r) => [r.fromIndex, r.toIndex])).toEqual([
      [0, 1],
      [1, 1],
      [1, 2],
      [0, 2],
    ])
  })
})

describe("groupEntries", () => {
  const entry = (label: string, kind: EntryPoint["kind"], node = "n"): EntryPoint => ({
    id: label,
    kind,
    label,
    node,
    symbolId: node,
    member: null,
    site: { file: "a.ts", line: 1 },
  })

  it("groups by kind and filters by label or component", () => {
    const entries = [entry("GET /a", "http", "Claims"), entry("tick", "cron"), entry("POST /b", "http")]
    expect(groupEntries(entries, "", (id) => id).map((g) => [g.label, g.entries.length])).toEqual([
      ["HTTP", 2],
      ["Scheduled", 1],
    ])
    expect(groupEntries(entries, "claims", (id) => id)[0].entries.map((e) => e.label)).toEqual(["GET /a"])
  })
})
