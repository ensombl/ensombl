import { describe, expect, it } from "vitest"

import type { SystemModel } from "@/shared/bindings/SystemModel"

import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { SymbolNode } from "@/shared/bindings/SymbolNode"

import { bracketedPaste, buildPrompt } from "./context"

const service: SymbolNode = {
  id: "src/user.ts#UserService",
  kind: "class",
  name: "UserService",
  file: "src/user.ts",
  range: { startLine: 3, endLine: 40 },
  package: "api",
  exported: true,
  isAbstract: false,
  typeParams: null,
  signature: "class UserService implements Service",
  members: [
    {
      name: "save",
      kind: "method",
      signature: "save(user: User): Promise<void>",
      visibility: "public",
      isStatic: false,
      isAsync: true,
      isAbstract: false,
      typeText: "Promise<void>",
      line: 12,
    },
  ],
}
const repo: SymbolNode = { ...service, id: "src/repo.ts#Repo", name: "Repo", file: "src/repo.ts", members: [] }
const model: ArchModel = {
  root: "/r",
  symbols: [service, repo],
  edges: [{ from: service.id, to: repo.id, kind: "association" }],
}

describe("buildPrompt", () => {
  it("returns the plain prompt without tags", () => {
    expect(buildPrompt("  refactor this  ", [], model)).toBe("refactor this")
  })

  it("appends symbol context with location, signature, relations and members", () => {
    const out = buildPrompt("add caching to @UserService", [{ type: "symbol", symbolId: service.id, label: "UserService" }], model)
    expect(out).toContain("add caching to @UserService")
    expect(out).toContain("### class UserService")
    expect(out).toContain("@src/user.ts:3-40")
    expect(out).toContain("has: Repo")
    expect(out).toContain("+ async save(user: User): Promise<void>  (line 12)")
  })

  it("describes members and files compactly", () => {
    const out = buildPrompt(
      "x",
      [
        { type: "member", symbolId: service.id, member: "save", label: "UserService.save" },
        { type: "file", path: "src/repo.ts", label: "repo.ts" },
      ],
      model,
    )
    expect(out).toContain("### UserService.save\n@src/user.ts:12")
    expect(out).toContain("### file\n@src/repo.ts")
  })

  it("lists incoming relations as used by", () => {
    const out = buildPrompt("x", [{ type: "symbol", symbolId: repo.id, label: "Repo" }], model)
    expect(out).toContain("used by: UserService")
  })
})

describe("pattern tags", () => {
  it("lists roles with locations", () => {
    const out = buildPrompt(
      "follow @pattern",
      [{ type: "pattern", patternId: "strategy:x", label: "Strategy/Repo" }],
      model,
      {
        patterns: [
        {
          id: "strategy:x",
          kind: "strategy",
          name: "Strategy: Repo",
          confidence: 0.9,
          evidence: ["2 implementations"],
          participants: [
            { symbolId: repo.id, role: "Strategy" },
            { symbolId: service.id, role: "Context" },
          ],
        },
        ],
      },
    )
    expect(out).toContain("### pattern Strategy: Repo (strategy, confidence 0.90)")
    expect(out).toContain("  - Strategy: Repo  @src/repo.ts:3")
    expect(out).toContain("  - Context: UserService  @src/user.ts:3")
  })
})

describe("component and flow tags", () => {
  const system: SystemModel = {
    nodes: [
      {
        id: "s",
        kind: "component",
        name: "ClaimsService",
        role: "service",
        layer: "application",
        package: "api",
        feature: "claims",
        symbolIds: ["s"],
        file: "src/claims.service.ts",
        line: 4,
        description: "Owns the claim lifecycle",
        hidden: false,
        merged: [],
      },
      { id: "ext:Database", kind: "external", name: "Database", role: "external", layer: "external", package: "", feature: "external", symbolIds: [], file: null, line: null, description: null, hidden: false, merged: [] },
    ],
    interactions: [{ id: "i", from: "s", to: "ext:Database", kind: "call", members: ["Claim.save"], sites: [], count: 1 }],
    entryPoints: [{ id: "e", kind: "http", label: "POST /claims", node: "s", symbolId: "s", member: "submit", site: { file: "src/claims.service.ts", line: 9 } }],
  }

  it("describes a component with the user's description and interactions", () => {
    const out = buildPrompt("x", [{ type: "component", componentId: "s", label: "component:ClaimsService" }], model, { system })
    expect(out).toContain("### component ClaimsService (service, application layer, feature claims)")
    expect(out).toContain("description: Owns the claim lifecycle")
    expect(out).toContain("talks to: Database (calls Claim.save)")
    expect(out).toContain("entry points: POST /claims")
  })

  it("lists a flow's traced steps by depth", () => {
    const flows = {
      e: [{ depth: 1, from: "s", to: "ext:Database", label: "Claim.save", kind: "call" as const, isAsync: false, site: { file: "src/claims.service.ts", line: 12 } }],
    }
    const out = buildPrompt("x", [{ type: "flow", entryId: "e", label: "flow:POST /claims" }], model, { system, flows })
    expect(out).toContain("### flow POST /claims (starts in ClaimsService.submit @src/claims.service.ts:9)")
    expect(out).toContain("  - ClaimsService → Database: Claim.save  @src/claims.service.ts:12")
  })
})

describe("bracketedPaste", () => {
  it("wraps text in paste markers", () => {
    expect(bracketedPaste("a\nb")).toBe("\x1b[200~a\nb\x1b[201~")
  })
})
