import { describe, expect, it } from "vitest"

import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { SymbolNode } from "@/shared/bindings/SymbolNode"

import { ALL_EDGE_KINDS, ancestorsOf, buildView, defaultExpanded, expandOneLevel, folderId, packageId, type ViewState } from "./view"

function sym(file: string, name: string, pkg: string): SymbolNode {
  return {
    id: `${file}#${name}`,
    kind: "class",
    name,
    file,
    range: { startLine: 1, endLine: 1 },
    package: pkg,
    exported: true,
    isAbstract: false,
    typeParams: null,
    signature: `class ${name}`,
    members: [],
  }
}

const model: ArchModel = {
  root: "/repo",
  symbols: [
    sym("packages/api/src/user/service.ts", "UserService", "api"),
    sym("packages/api/src/user/repo.ts", "UserRepo", "api"),
    sym("packages/api/src/db/client.ts", "Db", "api"),
    sym("packages/api/src/db/client.test.ts", "DbTest", "api"),
    sym("packages/ui/src/button.tsx", "Button", "ui"),
  ],
  edges: [
    { from: "packages/api/src/user/service.ts#UserService", to: "packages/api/src/user/repo.ts#UserRepo", kind: "association" },
    { from: "packages/api/src/user/service.ts#UserService", to: "packages/api/src/user/repo.ts#UserRepo", kind: "dependency" },
    { from: "packages/api/src/user/repo.ts#UserRepo", to: "packages/api/src/db/client.ts#Db", kind: "association" },
    { from: "packages/ui/src/button.tsx#Button", to: "packages/api/src/user/service.ts#UserService", kind: "dependency" },
    { from: "packages/api/src/db/client.test.ts#DbTest", to: "packages/api/src/db/client.ts#Db", kind: "dependency" },
  ],
}

const state = (expanded: string[], extra: Partial<ViewState> = {}): ViewState => ({
  expanded: new Set(expanded),
  open: new Set(),
  filters: { hideTests: false, edgeKinds: ALL_EDGE_KINDS },
  ...extra,
})

describe("buildView", () => {
  it("shows only packages when collapsed, with aggregated edges", () => {
    const view = buildView(model, state([]))
    expect(view.nodes.map((n) => n.id)).toEqual([packageId("api"), packageId("ui")])
    expect(view.edges).toEqual([{ id: "pkg:ui->pkg:api", source: "pkg:ui", target: "pkg:api", kind: "aggregate", count: 1 }])
  })

  it("shows folders inside an expanded package, labelled relative to the package", () => {
    const view = buildView(model, state([packageId("api")]))
    const folders = view.nodes.filter((n) => n.type === "group" && n.level === "folder")
    expect(folders.map((f) => f.type === "group" && f.label)).toEqual(["db", "user"])
    const userToDb = view.edges.find((e) => e.source.endsWith("/user") && e.target.endsWith("/db"))
    expect(userToDb?.kind).toBe("aggregate")
  })

  it("draws symbol edges once with the strongest kind", () => {
    const expanded = [packageId("api"), folderId("api", "packages/api/src/user"), folderId("api", "packages/api/src/db")]
    const view = buildView(model, state(expanded))
    const between = view.edges.filter((e) => e.source.endsWith("#UserService") && e.target.endsWith("#UserRepo"))
    expect(between).toHaveLength(1)
    expect(between[0].kind).toBe("association")
  })

  it("skips the folder level for single-folder packages", () => {
    const view = buildView(model, state([packageId("ui")]))
    const button = view.nodes.find((n) => n.id.endsWith("#Button"))
    expect(button?.parent).toBe(packageId("ui"))
  })

  it("filters test files and edge kinds", () => {
    const expanded = [packageId("api"), folderId("api", "packages/api/src/db")]
    const view = buildView(model, state(expanded, { filters: { hideTests: true, edgeKinds: ["association"] } }))
    expect(view.nodes.some((n) => n.id.endsWith("#DbTest"))).toBe(false)
    expect(view.edges.every((e) => e.kind === "aggregate" || e.kind === "association")).toBe(true)
  })
})

describe("helpers", () => {
  it("expands small models by default", () => {
    expect(defaultExpanded(model).has(packageId("api"))).toBe(true)
    expect(defaultExpanded(model, 2).size).toBe(0)
  })

  it("expands packages first, then folders", () => {
    const first = expandOneLevel(model, new Set())
    expect([...first].sort()).toEqual([packageId("api"), packageId("ui")])
    const second = expandOneLevel(model, first)
    expect(second.has(folderId("api", "packages/api/src/db"))).toBe(true)
  })

  it("finds the ancestors needed to reveal a symbol", () => {
    expect(ancestorsOf(model, "packages/api/src/db/client.ts#Db")).toEqual([
      packageId("api"),
      folderId("api", "packages/api/src/db"),
    ])
  })
})
