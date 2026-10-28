import { memo } from "react"

import { cn } from "@/lib/utils"
import type { SystemNode } from "@/shared/bindings/SystemNode"

import type { Sequence } from "../../model/sequence"
import { KIND_STYLE, LAYER_COLOR, NodeIcon } from "../components/system-style"

const COLUMN = 190
const HEADER = 56
const ROW = 34
const SELF_LOOP = 26

interface SequenceDiagramProps {
  sequence: Sequence
  nodes: Map<string, SystemNode>
  onStep: (file: string, line: number) => void
}

/**
 * Lifelines with one arrow per step; async steps are dashed, self-calls loop
 * back. The participant row is sticky, so it stays pinned while you scroll.
 * Rows are positioned from `HEADER` down, inside the same scroll container.
 */
export const SequenceDiagram = memo(function SequenceDiagram({ sequence, nodes, onStep }: SequenceDiagramProps) {
  const width = sequence.participants.length * COLUMN
  const height = HEADER + sequence.rows.length * ROW + 24
  const center = (i: number) => i * COLUMN + COLUMN / 2

  return (
    <div className="relative" style={{ width, height }}>
      <div className="sticky top-0 z-10 bg-background" style={{ width, height: HEADER }}>
        {sequence.participants.map((id, i) => {
          const node = nodes.get(id)
          return (
            <div
              key={id}
              className="absolute top-2 flex h-10 items-center gap-1.5 rounded-md border bg-card px-2 text-xs shadow-xs"
              style={{
                left: i * COLUMN + 8,
                width: COLUMN - 16,
                borderTopColor: node ? LAYER_COLOR[node.layer] : undefined,
                borderTopWidth: 2,
              }}
              title={node?.description ?? undefined}
            >
              {node && <NodeIcon node={node} className="size-3.5 shrink-0 text-muted-foreground" />}
              <span className="truncate font-medium">{node?.name ?? id.split(/[#:]/).pop()}</span>
            </div>
          )
        })}
      </div>
      <svg className="absolute inset-0" width={width} height={height}>
        <defs>
          {Object.entries(KIND_STYLE).map(([kind, style]) => (
            <marker key={kind} id={`seq-${kind}`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto">
              <path d="M 0 0 L 10 5 L 0 10 z" fill={style.color} />
            </marker>
          ))}
        </defs>
        {sequence.participants.map((id, i) => (
          <line key={id} x1={center(i)} x2={center(i)} y1={HEADER} y2={height} className="stroke-border" strokeDasharray="3 4" />
        ))}
        {sequence.rows.map((row, r) => {
          const y = HEADER + r * ROW + ROW / 2 + 6
          const style = KIND_STYLE[row.kind]
          const x1 = center(row.fromIndex)
          const x2 = center(row.toIndex)
          const path =
            row.fromIndex === row.toIndex
              ? `M ${x1} ${y - 8} h ${SELF_LOOP} v 12 h ${-SELF_LOOP + 4}`
              : `M ${x1} ${y} L ${x2 + (x2 > x1 ? -4 : 4)} ${y}`
          return (
            <path
              key={r}
              d={path}
              fill="none"
              stroke={style.color}
              strokeWidth={1.5}
              strokeDasharray={row.isAsync ? "4 4" : undefined}
              markerEnd={`url(#seq-${row.kind})`}
              opacity={Math.max(1 - row.depth * 0.08, 0.55)}
            />
          )
        })}
      </svg>

      {sequence.rows.map((row, r) => {
        const self = row.fromIndex === row.toIndex
        const left = self ? center(row.fromIndex) + SELF_LOOP + 6 : Math.min(center(row.fromIndex), center(row.toIndex)) + 8
        const span = self ? COLUMN - SELF_LOOP - 16 : Math.abs(center(row.toIndex) - center(row.fromIndex)) - 16
        return (
          <button
            key={r}
            type="button"
            onClick={() => onStep(row.site.file, row.site.line)}
            title={`${row.label}\n${row.site.file}:${row.site.line}`}
            className={cn(
              "absolute truncate rounded px-1 text-left font-mono text-[10px] text-muted-foreground hover:bg-accent hover:text-foreground",
              self && "text-muted-foreground/80",
            )}
            style={{ top: HEADER + r * ROW + (self ? 6 : 0), left, maxWidth: Math.max(span, 60) }}
          >
            {row.label}
          </button>
        )
      })}
    </div>
  )
})
