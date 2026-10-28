import { memo } from "react"
import { Handle, Position, type Node, type NodeProps } from "@xyflow/react"

import { cn } from "@/lib/utils"
import type { SymbolNode as Symbol } from "@/shared/bindings/SymbolNode"

import { packageColor } from "../../model/package-colors"
import { KindBadge } from "../kind-badge"

export type SymbolFlowNode = Node<{ symbol: Symbol; pkg: string }, "symbol">

/** A compact class/interface box. Members are shown in the inspector, not on the map. */
function SymbolNodeView({ data, selected }: NodeProps<SymbolFlowNode>) {
  const { symbol } = data
  return (
    <div
      title={`${symbol.signature}\n${symbol.file}:${symbol.range.startLine}`}
      className={cn(
        "flex h-full w-full items-center gap-2 rounded-lg border border-l-[3px] bg-card px-2.5 text-card-foreground shadow-sm transition-colors hover:bg-accent/40",
        symbol.kind === "interface" && "border-dashed",
        selected && "ring-2 ring-ring",
      )}
      style={{ borderLeftColor: packageColor(data.pkg).value, borderLeftStyle: "solid" }}
    >
      <Handle type="source" position={Position.Top} className="!opacity-0" />
      <Handle type="target" position={Position.Bottom} className="!opacity-0" />
      <KindBadge kind={symbol.kind} />
      <span className={cn("min-w-0 flex-1 truncate text-sm font-medium", symbol.isAbstract && "italic")}>
        {symbol.name}
        {symbol.typeParams && <span className="font-mono text-xs font-normal text-muted-foreground">{symbol.typeParams}</span>}
      </span>
      {symbol.members.length > 0 && (
        <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{symbol.members.length}</span>
      )}
    </div>
  )
}

export const SymbolNodeComponent = memo(SymbolNodeView)
