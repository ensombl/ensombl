import { describe, expect, it } from "vitest"

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

describe("bracketedPaste", () => {
  it("wraps text in paste markers", () => {
    expect(bracketedPaste("a\nb")).toBe("\x1b[200~a\nb\x1b[201~")
  })
})
