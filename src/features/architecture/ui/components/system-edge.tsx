import { memo } from "react"
import { BaseEdge, EdgeLabelRenderer, getSmoothStepPath, type Edge, type EdgeProps } from "@xyflow/react"

import type { Point } from "../../layout/elk-graph"
import { roundedPath } from "../../layout/paths"
import { membersLabel, type ComponentsViewEdge } from "../../model/system-view"
import { KIND_STYLE, markerFor } from "./system-style"

export type SystemFlowEdge = Edge<
  {
    kind: ComponentsViewEdge["kind"]
    members: string[]
    count: number
    highlighted: boolean
    dimmed: boolean
    showLabel: boolean
    /** Lots of edges on screen: draw the rest faintly so highlighted ones read. */
    busy: boolean
    points?: Point[]
  },
  "system"
>

export const SystemEdgeView = memo(function SystemEdgeView({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
}: EdgeProps<SystemFlowEdge>) {
  const points = data?.points
  const [path, labelX, labelY] =
    points && points.length >= 2
      ? roundedPath(points)
      : getSmoothStepPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, borderRadius: 10 })
  const kind = data?.kind ?? "call"
  const style = KIND_STYLE[kind]
  const count = data?.count ?? 1
  const width = kind === "aggregate" ? Math.min(1.25 + Math.log2(count) * 0.5, 4.5) : style.width
  const label = kind === "aggregate" ? String(count) : membersLabel(data?.members ?? [])

  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={`url(#${markerFor(kind)})`}
        style={{
          stroke: style.color,
          strokeOpacity: data?.highlighted ? 1 : data?.dimmed ? 0.08 : data?.busy ? 0.22 : 0.55,
          strokeWidth: data?.highlighted ? width + 0.75 : width,
          strokeDasharray: style.dash,
        }}
      />
      {label && (data?.showLabel || data?.highlighted) && (
        <EdgeLabelRenderer>
          <div
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
            className="pointer-events-none absolute max-w-48 truncate rounded border bg-background/95 px-1.5 py-px font-mono text-[10px] text-foreground"
          >
            {label}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  )
})
