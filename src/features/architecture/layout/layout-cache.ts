/**
 * Recent layouts by signature. Lenses unmount when hidden, so coming back to
 * one reuses its layout instead of waiting on ELK again.
 */
export function layoutCache<T>(capacity = 12) {
  const entries = new Map<string, T>()
  return {
    get: (key: string) => {
      const value = entries.get(key)
      if (value !== undefined) {
        // Refresh recency.
        entries.delete(key)
        entries.set(key, value)
      }
      return value
    },
    set: (key: string, value: T) => {
      entries.delete(key)
      entries.set(key, value)
      if (entries.size > capacity) entries.delete(entries.keys().next().value!)
    },
  }
}
