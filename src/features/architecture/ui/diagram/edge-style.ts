import type { ViewEdge } from "../../model/view"

/** UML notation plus a color per relation kind. */
export const EDGE_STYLE: Record<ViewEdge["kind"], { color: string; marker: "triangle" | "arrow"; dashed: boolean; label: string }> = {
  extends: { color: "oklch(0.72 0.15 250)", marker: "triangle", dashed: false, label: "Extends" },
  implements: { color: "oklch(0.74 0.15 160)", marker: "triangle", dashed: true, label: "Implements" },
  association: { color: "oklch(0.77 0.15 75)", marker: "arrow", dashed: false, label: "Has" },
  dependency: { color: "oklch(0.70 0.01 286)", marker: "arrow", dashed: true, label: "Uses" },
  aggregate: { color: "oklch(0.62 0.01 286)", marker: "arrow", dashed: false, label: "Depends on" },
}

export const markerId = (kind: ViewEdge["kind"]) => `uml-${kind}`
