import { memo } from "react"
import { BaseEdge, EdgeLabelRenderer, getSmoothStepPath, type Edge, type EdgeProps } from "@xyflow/react"

import { cn } from "@/lib/utils"

import type { ViewEdge } from "../../model/view"

export type UmlFlowEdge = Edge<{ kind: ViewEdge["kind"]; count: number; highlighted: boolean }, "uml">

/** UML notation: hollow triangle for generalization/realization, open arrow otherwise; dashed for realization and dependency. */
const STYLE: Record<ViewEdge["kind"], { marker: string; dashed: boolean }> = {
  extends: { marker: "uml-triangle", dashed: false },
  implements: { marker: "uml-triangle", dashed: true },
  association: { marker: "uml-arrow", dashed: false },
  dependency: { marker: "uml-arrow", dashed: true },
  aggregate: { marker: "uml-arrow", dashed: false },
}

function UmlEdgeView({ id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data }: EdgeProps<UmlFlowEdge>) {
  const [path, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    borderRadius: 10,
  })
  const kind = data?.kind ?? "dependency"
  const style = STYLE[kind]
  const highlighted = data?.highlighted ?? false
  const width = kind === "aggregate" ? Math.min(1 + Math.log2(data?.count ?? 1), 5) : 1.25

  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={`url(#${style.marker}${highlighted ? "-active" : ""})`}
        style={{
          stroke: highlighted ? "var(--primary)" : "var(--muted-foreground)",
          strokeOpacity: highlighted ? 1 : 0.45,
          strokeWidth: highlighted ? width + 0.75 : width,
          strokeDasharray: style.dashed ? "6 4" : undefined,
        }}
      />
      {kind === "aggregate" && (data?.count ?? 0) > 1 && (
        <EdgeLabelRenderer>
          <div
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
            className={cn(
              "pointer-events-none absolute rounded-full border bg-background px-1.5 font-mono text-[10px]",
              highlighted ? "border-primary text-foreground" : "text-muted-foreground",
            )}
          >
            {data?.count}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  )
}

export const UmlEdgeComponent = memo(UmlEdgeView)

/** SVG marker definitions referenced by UML edges. Render once per canvas. */
export function UmlMarkers() {
  const markers = [
    { id: "uml-triangle", color: "var(--muted-foreground)", triangle: true },
    { id: "uml-triangle-active", color: "var(--primary)", triangle: true },
    { id: "uml-arrow", color: "var(--muted-foreground)", triangle: false },
    { id: "uml-arrow-active", color: "var(--primary)", triangle: false },
  ]
  return (
    <svg className="pointer-events-none absolute size-0">
      <defs>
        {markers.map((m) => (
          <marker
            key={m.id}
            id={m.id}
            viewBox="0 0 12 12"
            refX="11"
            refY="6"
            markerWidth="12"
            markerHeight="12"
            markerUnits="userSpaceOnUse"
            orient="auto-start-reverse"
          >
            {m.triangle ? (
              <path d="M1,1 L11,6 L1,11 Z" fill="var(--background)" stroke={m.color} strokeWidth="1.25" />
            ) : (
              <path d="M2,2 L11,6 L2,10" fill="none" stroke={m.color} strokeWidth="1.25" />
            )}
          </marker>
        ))}
      </defs>
    </svg>
  )
}
