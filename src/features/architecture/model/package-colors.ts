/** A stable accent per package so you can tell areas of the codebase apart. */
export interface PackageColor {
  /** CSS color for strokes, minimap and inline styles. */
  value: string
  /** Translucent fill. */
  tint: string
}

const PALETTE = [
  "oklch(0.72 0.15 250)", // blue
  "oklch(0.74 0.15 160)", // emerald
  "oklch(0.75 0.15 75)", // amber
  "oklch(0.70 0.18 300)", // violet
  "oklch(0.72 0.17 0)", // rose
  "oklch(0.76 0.12 200)", // cyan
  "oklch(0.74 0.17 40)", // orange
  "oklch(0.78 0.15 125)", // lime
  "oklch(0.72 0.16 330)", // pink
  "oklch(0.72 0.12 275)", // indigo
]

function hash(text: string): number {
  let h = 2166136261
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  return h >>> 0
}

export function packageColor(pkg: string | undefined): PackageColor {
  const value = PALETTE[hash(pkg ?? "") % PALETTE.length]
  return { value, tint: value.replace(")", " / 9%)") }
}
