import { memo } from "react"
import { Handle, Position, type Node, type NodeProps } from "@xyflow/react"
import { ChevronDown, ChevronRight, Folder, Package } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"

import { SIZES } from "../../layout/sizes"
import { useDiagram } from "../../model/diagram-store"

export type GroupFlowNode = Node<
  { label: string; level: "package" | "folder"; expanded: boolean; symbolCount: number },
  "group"
>

function GroupNodeView({ id, data, selected }: NodeProps<GroupFlowNode>) {
  const { toggleGroup } = useDiagram()
  const Icon = data.level === "package" ? Package : Folder
  const toggle = () => toggleGroup(id)

  if (data.expanded) {
    return (
      <div
        className={cn(
          "h-full w-full rounded-xl border",
          data.level === "package" ? "border-border bg-muted/20" : "border-dashed border-border/70 bg-muted/10",
          selected && "ring-2 ring-ring",
        )}
      >
        <Handle type="source" position={Position.Top} className="!opacity-0" />
        <Handle type="target" position={Position.Bottom} className="!opacity-0" />
        <button
          type="button"
          onClick={toggle}
          style={{ height: SIZES.groupPadding.top - 8 }}
          className="nodrag flex max-w-full items-center gap-2 px-3 text-sm text-muted-foreground hover:text-foreground"
        >
          <ChevronDown className="size-3.5 shrink-0" />
          <Icon className="size-4 shrink-0" />
          <span className={cn("truncate", data.level === "package" ? "font-semibold text-foreground" : "font-mono text-xs")}>
            {data.label}
          </span>
          <span className="text-xs">{data.symbolCount}</span>
        </button>
      </div>
    )
  }

  return (
    <div
      onDoubleClick={toggle}
      className={cn(
        "flex h-full w-full items-center gap-3 rounded-lg border bg-card px-3 text-card-foreground shadow-sm",
        data.level === "folder" && "border-dashed",
        selected && "ring-2 ring-ring",
      )}
    >
      <Handle type="source" position={Position.Top} className="!opacity-0" />
      <Handle type="target" position={Position.Bottom} className="!opacity-0" />
      <div className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
        <Icon className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <p className={cn("truncate text-sm", data.level === "package" ? "font-semibold" : "font-mono text-xs")}>{data.label}</p>
        <Badge variant="secondary" className="mt-1 h-5 px-1.5 text-[10px]">
          {data.symbolCount} symbols
        </Badge>
      </div>
      <button
        type="button"
        aria-label="Expand"
        onClick={toggle}
        className="nodrag rounded p-1 text-muted-foreground hover:bg-accent hover:text-accent-foreground"
      >
        <ChevronRight className="size-4" />
      </button>
    </div>
  )
}

export const GroupNodeComponent = memo(GroupNodeView)
