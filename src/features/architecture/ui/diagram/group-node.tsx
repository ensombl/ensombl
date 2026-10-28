import { memo } from "react"
import { Handle, Position, type Node, type NodeProps } from "@xyflow/react"
import { ChevronDown, ChevronRight, Folder, Package } from "lucide-react"

import { cn } from "@/lib/utils"

import { SIZES } from "../../layout/sizes"
import { useDiagram } from "../../model/diagram-store"
import { packageColor } from "../../model/package-colors"
import { ChangeMarker } from "./change-marker"

export type GroupFlowNode = Node<
  {
    label: string
    title: string
    pkg: string
    level: "package" | "folder"
    expanded: boolean
    symbolCount: number
    changedAt?: number
  },
  "group"
>

function GroupNodeView({ id, data, selected }: NodeProps<GroupFlowNode>) {
  const { toggleGroup } = useDiagram()
  const color = packageColor(data.pkg)
  const isPackage = data.level === "package"
  const Icon = isPackage ? Package : Folder
  const toggle = () => toggleGroup(id)

  if (data.expanded) {
    return (
      <div
        className={cn("h-full w-full rounded-xl border", !isPackage && "border-dashed")}
        style={{
          borderColor: `color-mix(in oklch, ${color.value} ${isPackage ? 45 : 28}%, transparent)`,
          background: isPackage ? color.tint : "color-mix(in oklch, var(--muted) 18%, transparent)",
        }}
      >
        <Handle type="source" position={Position.Top} className="!opacity-0" />
        <Handle type="target" position={Position.Bottom} className="!opacity-0" />
        <button
          type="button"
          onClick={toggle}
          title={data.title}
          style={{ height: SIZES.groupPadding.top - 8 }}
          className="nodrag flex max-w-full items-center gap-2 px-3 text-sm hover:opacity-80"
        >
          <ChevronDown className="size-3.5 shrink-0 text-muted-foreground" />
          <Icon className="size-4 shrink-0" style={{ color: color.value }} />
          <span className={cn("truncate", isPackage ? "font-semibold" : "font-mono text-xs")} style={isPackage ? { color: color.value } : undefined}>
            {data.label}
          </span>
          <span className="font-mono text-xs text-muted-foreground">{data.symbolCount}</span>
        </button>
      </div>
    )
  }

  return (
    <div
      onDoubleClick={toggle}
      title={data.title}
      className={cn(
        "relative flex h-full w-full items-center gap-2.5 rounded-lg border border-l-4 bg-card px-2.5 text-card-foreground shadow-sm transition-colors hover:bg-accent/40",
        !isPackage && "border-dashed",
        selected && "ring-2 ring-ring",
      )}
      style={{ borderLeftColor: color.value, borderLeftStyle: "solid" }}
    >
      <Handle type="source" position={Position.Top} className="!opacity-0" />
      <Handle type="target" position={Position.Bottom} className="!opacity-0" />
      <ChangeMarker changedAt={data.changedAt} />
      <div className="flex size-7 shrink-0 items-center justify-center rounded-md" style={{ background: color.tint, color: color.value }}>
        <Icon className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <p className={cn("truncate leading-tight", isPackage ? "text-sm font-semibold" : "font-mono text-xs font-medium")}>{data.label}</p>
        <p className="text-[11px] leading-tight text-muted-foreground">{data.symbolCount} symbols</p>
      </div>
      <button
        type="button"
        aria-label="Expand"
        onClick={(e) => {
          e.stopPropagation()
          toggle()
        }}
        className="nodrag rounded p-1 text-muted-foreground hover:bg-accent hover:text-accent-foreground"
      >
        <ChevronRight className="size-4" />
      </button>
    </div>
  )
}

export const GroupNodeComponent = memo(GroupNodeView)
