import { memo } from "react"
import { Handle, Position, type Node, type NodeProps } from "@xyflow/react"
import { DoorOpen } from "lucide-react"

import { cn } from "@/lib/utils"
import type { Layer } from "@/shared/bindings/Layer"
import type { Role } from "@/shared/bindings/Role"
import type { SystemNode } from "@/shared/bindings/SystemNode"

import { LAYER_LABEL } from "../../layout/system-layout"
import { packageColor } from "../../model/package-colors"
import { FeatureIcon, LAYER_COLOR, NodeIcon, ROLE_LABEL } from "./system-style"

const handles = (
  <>
    <Handle type="target" position={Position.Left} className="!size-1 !min-h-0 !min-w-0 !border-0 !bg-transparent" />
    <Handle type="source" position={Position.Right} className="!size-1 !min-h-0 !min-w-0 !border-0 !bg-transparent" />
  </>
)

export type ComponentFlowNode = Node<
  { node: SystemNode; entryCount: number; context: boolean; dimmed: boolean },
  "component"
>

export const ComponentNodeView = memo(function ComponentNodeView({ data, selected }: NodeProps<ComponentFlowNode>) {
  const { node, entryCount, context, dimmed } = data
  const color = LAYER_COLOR[node.layer]
  const special = node.kind !== "component"
  return (
    <div
      className={cn(
        "flex h-full w-full flex-col justify-center gap-0.5 rounded-lg border bg-card px-2.5 text-card-foreground shadow-xs transition-opacity",
        special && "rounded-full px-4",
        node.kind === "channel" && "border-dashed",
        selected && "ring-2 ring-ring",
        (context || node.hidden) && "opacity-60",
        dimmed && "opacity-25",
      )}
      style={{ borderLeftColor: special ? undefined : color, borderLeftWidth: special ? undefined : 3 }}
    >
      {handles}
      <div className="flex min-w-0 items-center gap-1.5">
        <NodeIcon node={node} className="size-3.5 shrink-0" />
        <span className="truncate text-xs font-medium">{node.name}</span>
        {entryCount > 0 && (
          <span className="ml-auto flex shrink-0 items-center gap-0.5 font-mono text-[10px] text-muted-foreground" title="Entry points">
            <DoorOpen className="size-3" />
            {entryCount}
          </span>
        )}
      </div>
      <p className="truncate text-[11px] text-muted-foreground">
        {node.description || (context ? `${node.feature} · ${ROLE_LABEL[node.role]}` : ROLE_LABEL[node.role])}
      </p>
    </div>
  )
})

export type FeatureFlowNode = Node<
  { label: string; pkg: string; count: number; entryCount: number; roles: Partial<Record<Role, number>>; dimmed: boolean },
  "feature"
>

const TOP_ROLES = 3

export const FeatureNodeView = memo(function FeatureNodeView({ data, selected }: NodeProps<FeatureFlowNode>) {
  const color = packageColor(`${data.pkg}/${data.label}`)
  const roles = Object.entries(data.roles)
    .sort((a, b) => (b[1] ?? 0) - (a[1] ?? 0))
    .slice(0, TOP_ROLES)
  return (
    <div
      className={cn(
        "flex h-full w-full flex-col justify-center gap-1 rounded-xl border-2 bg-card px-3 shadow-xs",
        selected && "ring-2 ring-ring",
        data.dimmed && "opacity-25",
      )}
      style={{ borderColor: color.value, background: color.tint }}
    >
      {handles}
      <div className="flex min-w-0 items-center gap-1.5">
        <FeatureIcon className="size-4 shrink-0" style={{ color: color.value }} />
        <span className="truncate text-sm font-semibold">{data.label}</span>
        <span className="ml-auto shrink-0 font-mono text-[10px] text-muted-foreground">{data.pkg}</span>
      </div>
      <p className="truncate text-[11px] text-muted-foreground">
        {data.count} components{data.entryCount > 0 && ` · ${data.entryCount} entry points`}
      </p>
      <p className="truncate text-[10px] text-muted-foreground/80">
        {roles.map(([role, n]) => `${n} ${ROLE_LABEL[role as Role].toLowerCase()}`).join(" · ")}
      </p>
    </div>
  )
})

export type LaneFlowNode = Node<{ layer: Layer }, "lane">

export const LaneNodeView = memo(function LaneNodeView({ data }: NodeProps<LaneFlowNode>) {
  return (
    <div className="h-full w-full rounded-xl border border-dashed border-border/60 bg-muted/10">
      <p className="px-3 pt-2 text-[11px] font-medium tracking-wide uppercase" style={{ color: LAYER_COLOR[data.layer] }}>
        {LAYER_LABEL[data.layer]}
      </p>
    </div>
  )
})
