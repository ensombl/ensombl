import { memo } from "react"
import { BaseEdge, EdgeLabelRenderer, getSmoothStepPath, type Edge, type EdgeProps } from "@xyflow/react"

import type { Point } from "../../layout/elk-graph"
import type { ViewEdge } from "../../model/view"
import { EDGE_STYLE, markerId } from "./edge-style"

export type UmlFlowEdge = Edge<
  { kind: ViewEdge["kind"]; count: number; highlighted: boolean; dimmed: boolean; points?: Point[] },
  "uml"
>

/** Orthogonal polyline with rounded corners through ELK's route points. */
function roundedPath(points: Point[], radius = 6): [string, number, number] {
  let d = `M ${points[0].x} ${points[0].y}`
  for (let i = 1; i < points.length - 1; i++) {
    const [prev, cur, next] = [points[i - 1], points[i], points[i + 1]]
    const r = Math.min(radius, Math.hypot(cur.x - prev.x, cur.y - prev.y) / 2, Math.hypot(next.x - cur.x, next.y - cur.y) / 2)
    const inX = cur.x - Math.sign(cur.x - prev.x) * r
    const inY = cur.y - Math.sign(cur.y - prev.y) * r
    const outX = cur.x + Math.sign(next.x - cur.x) * r
    const outY = cur.y + Math.sign(next.y - cur.y) * r
    d += ` L ${inX} ${inY} Q ${cur.x} ${cur.y} ${outX} ${outY}`
  }
  const last = points[points.length - 1]
  d += ` L ${last.x} ${last.y}`
  const mid = points[Math.floor(points.length / 2)]
  return [d, mid.x, mid.y]
}

function UmlEdgeView({ id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data }: EdgeProps<UmlFlowEdge>) {
  const points = data?.points
  const [path, labelX, labelY] =
    points && points.length >= 2
      ? roundedPath(points)
      : getSmoothStepPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, borderRadius: 10 })
  const kind = data?.kind ?? "dependency"
  const style = EDGE_STYLE[kind]
  const highlighted = data?.highlighted ?? false
  const count = data?.count ?? 1
  const width = kind === "aggregate" ? Math.min(1.25 + Math.log2(count) * 0.6, 4) : 1.5

  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={`url(#${markerId(kind)})`}
        style={{
          stroke: style.color,
          strokeOpacity: highlighted ? 1 : data?.dimmed ? 0.12 : 0.6,
          strokeWidth: highlighted ? width + 1 : width,
          strokeDasharray: style.dashed ? "6 4" : undefined,
        }}
      />
      {kind === "aggregate" && count > 1 && highlighted && (
        <EdgeLabelRenderer>
          <div
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
            className="pointer-events-none absolute rounded-full border bg-background px-1.5 font-mono text-[10px] text-foreground"
          >
            {count}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  )
}

export const UmlEdgeComponent = memo(UmlEdgeView)

/** SVG marker definitions referenced by UML edges. Render once per canvas. */
export function UmlMarkers() {
  return (
    <svg className="pointer-events-none absolute size-0">
      <defs>
        {Object.entries(EDGE_STYLE).map(([kind, style]) => (
          <marker
            key={kind}
            id={markerId(kind as ViewEdge["kind"])}
            viewBox="0 0 12 12"
            refX="11"
            refY="6"
            markerWidth="12"
            markerHeight="12"
            markerUnits="userSpaceOnUse"
            orient="auto-start-reverse"
          >
            {style.marker === "triangle" ? (
              <path d="M1,1 L11,6 L1,11 Z" fill="var(--background)" stroke={style.color} strokeWidth="1.5" />
            ) : (
              <path d="M2,2 L11,6 L2,10" fill="none" stroke={style.color} strokeWidth="1.5" />
            )}
          </marker>
        ))}
      </defs>
    </svg>
  )
}
