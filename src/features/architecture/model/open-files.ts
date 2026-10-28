/** Open-file tabs in the Code tab: a file opens once, next to the one you're on. */
export function openTab(tabs: string[], active: string | null, path: string): string[] {
  if (tabs.includes(path)) return tabs
  const at = active ? tabs.indexOf(active) : -1
  return at < 0 ? [...tabs, path] : [...tabs.slice(0, at + 1), path, ...tabs.slice(at + 1)]
}

/** Closing the active tab moves to its right-hand neighbour, else the left. */
export function closeTab(tabs: string[], active: string | null, path: string): { tabs: string[]; active: string | null } {
  const at = tabs.indexOf(path)
  const next = tabs.filter((t) => t !== path)
  if (active !== path) return { tabs: next, active }
  return { tabs: next, active: next[Math.min(at, next.length - 1)] ?? null }
}
