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

const filters = { hideTests: false, showFunctions: true, edgeKinds: ALL_EDGE_KINDS, allGroupEdges: true }

const state = (expanded: string[], extra: Partial<ViewState> = {}): ViewState => ({
  expanded: new Set(expanded),
  filters,
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
    const view = buildView(model, state(expanded, { filters: { ...filters, hideTests: true, edgeKinds: ["association"] } }))
    expect(view.nodes.some((n) => n.id.endsWith("#DbTest"))).toBe(false)
    expect(view.edges.every((e) => e.kind === "aggregate" || e.kind === "association")).toBe(true)
  })
})

describe("nested folders", () => {
  const deep: ArchModel = {
    root: "/r",
    symbols: [
      sym("app/src/modules/users/http/controller.ts", "UsersController", "app"),
      sym("app/src/modules/users/service.ts", "UsersService", "app"),
      sym("app/src/modules/claims/service.ts", "ClaimsService", "app"),
      sym("app/scripts/seed.ts", "Seeder", "app"),
    ],
    edges: [
      { from: "app/src/modules/users/http/controller.ts#UsersController", to: "app/src/modules/claims/service.ts#ClaimsService", kind: "dependency" },
    ],
  }

  it("nests folders and compresses single-child chains", () => {
    const view = buildView(deep, state([packageId("app")]))
    const labels = view.nodes.filter((n) => n.type === "group" && n.level === "folder").map((n) => n.type === "group" && n.label)
    expect(labels).toEqual(["scripts", "src/modules"])
    const modules = folderId("app", "app/src/modules")
    const inside = buildView(deep, state([packageId("app"), modules]))
    const children = inside.nodes.filter((n) => n.parent === modules).map((n) => n.type === "group" && n.label)
    expect(children).toEqual(["claims", "users"])
  })

  it("counts every symbol below a folder", () => {
    const view = buildView(deep, state([packageId("app")]))
    const modules = view.nodes.find((n) => n.id === folderId("app", "app/src/modules"))
    expect(modules?.type === "group" && modules.symbolCount).toBe(3)
  })

  it("shows group edges only for the selected node unless all are requested", () => {
    const expanded = [packageId("app"), folderId("app", "app/src/modules")]
    const users = folderId("app", "app/src/modules/users")
    const quiet = buildView(deep, state(expanded, { filters: { ...filters, allGroupEdges: false } }))
    expect(quiet.edges).toHaveLength(0)
    const focused = buildView(deep, state(expanded, { filters: { ...filters, allGroupEdges: false }, selected: users }))
    expect(focused.edges.map((e) => e.source)).toEqual([users])
  })

  it("expands one visible level at a time", () => {
    const one = expandOneLevel(deep, filters, new Set())
    expect([...one]).toEqual([packageId("app")])
    const two = expandOneLevel(deep, filters, one)
    expect(two.has(folderId("app", "app/src/modules"))).toBe(true)
    expect(two.has(folderId("app", "app/src/modules/users"))).toBe(false)
  })
})

describe("symbol filters", () => {
  it("hides free functions and type aliases unless asked", () => {
    const withFn: ArchModel = { ...model, symbols: [...model.symbols, { ...sym("packages/ui/src/util.ts", "helper", "ui"), kind: "function" }] }
    const hidden = buildView(withFn, state([packageId("ui")], { filters: { ...filters, showFunctions: false } }))
    expect(hidden.nodes.some((n) => n.id.endsWith("#helper"))).toBe(false)
    const shown = buildView(withFn, state([packageId("ui")]))
    expect(shown.nodes.some((n) => n.id.endsWith("#helper"))).toBe(true)
  })
})

describe("helpers", () => {
  it("expands small models by default", () => {
    expect(defaultExpanded(model, filters).has(packageId("api"))).toBe(true)
    expect(defaultExpanded(model, filters, 2).size).toBe(0)
  })

  it("expands packages first, then folders", () => {
    const first = expandOneLevel(model, filters, new Set())
    expect([...first].sort()).toEqual([packageId("api"), packageId("ui")])
    const second = expandOneLevel(model, filters, first)
    expect(second.has(folderId("api", "packages/api/src/db"))).toBe(true)
  })

  it("finds the ancestors needed to reveal a symbol", () => {
    expect(ancestorsOf(model, "packages/api/src/db/client.ts#Db")).toEqual([
      packageId("api"),
      folderId("api", "packages/api/src/db"),
    ])
  })
})
