/** A folder or file in the explorer. Folders list subfolders first, then files, by name. */
export interface FileTreeNode {
  name: string
  /** Repo-relative path. */
  path: string
  children?: FileTreeNode[]
}

export function buildFileTree(paths: string[]): FileTreeNode[] {
  const root: FileTreeNode = { name: "", path: "", children: [] }
  const folders = new Map<string, FileTreeNode>([["", root]])
  for (const path of paths) {
    const parts = path.split("/")
    let parent = root
    for (let i = 0; i < parts.length - 1; i++) {
      const dir = parts.slice(0, i + 1).join("/")
      let folder = folders.get(dir)
      if (!folder) {
        folder = { name: parts[i], path: dir, children: [] }
        folders.set(dir, folder)
        parent.children!.push(folder)
      }
      parent = folder
    }
    parent.children!.push({ name: parts[parts.length - 1], path })
  }
  const sort = (nodes: FileTreeNode[]) => {
    nodes.sort((a, b) => Number(!a.children) - Number(!b.children) || a.name.localeCompare(b.name))
    for (const n of nodes) if (n.children) sort(n.children)
  }
  sort(root.children!)
  return root.children!
}

/** Folders that must be open for `path` to show. */
export const ancestorDirs = (path: string) =>
  path
    .split("/")
    .slice(0, -1)
    .map((_, i, parts) => parts.slice(0, i + 1).join("/"))

/** Paths matching every word of `query`, best (file name) matches first. */
export function searchFiles(paths: string[], query: string, limit = 200): string[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  if (words.length === 0) return []
  const scored: [number, string][] = []
  for (const p of paths) {
    const lower = p.toLowerCase()
    if (!words.every((w) => lower.includes(w))) continue
    const name = lower.slice(lower.lastIndexOf("/") + 1)
    scored.push([words.every((w) => name.includes(w)) ? 0 : 1, p])
  }
  return scored
    .sort((a, b) => a[0] - b[0] || a[1].length - b[1].length)
    .slice(0, limit)
    .map(([, p]) => p)
}
