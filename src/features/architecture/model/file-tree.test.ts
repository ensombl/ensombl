import { describe, expect, it } from "vitest"

import { ancestorDirs, buildFileTree, searchFiles } from "./file-tree"

describe("buildFileTree", () => {
  it("nests folders before files, sorted by name", () => {
    const tree = buildFileTree(["src/b.ts", "README.md", "src/lib/a.ts", "src/a.ts"])
    expect(tree.map((n) => n.name)).toEqual(["src", "README.md"])
    expect(tree[0].children!.map((n) => n.name)).toEqual(["lib", "a.ts", "b.ts"])
    expect(tree[0].children![0].children![0]).toEqual({ name: "a.ts", path: "src/lib/a.ts" })
  })
})

describe("file search", () => {
  it("matches every word and ranks file-name hits first", () => {
    const paths = ["src/claims/service.ts", "src/claims.ts", "docs/claims-guide.md"]
    expect(searchFiles(paths, "claims")).toEqual(["src/claims.ts", "docs/claims-guide.md", "src/claims/service.ts"])
    expect(searchFiles(paths, "claims service")).toEqual(["src/claims/service.ts"])
    expect(searchFiles(paths, "  ")).toEqual([])
  })

  it("lists the folders to open for a file", () => {
    expect(ancestorDirs("a/b/c.ts")).toEqual(["a", "a/b"])
  })
})
