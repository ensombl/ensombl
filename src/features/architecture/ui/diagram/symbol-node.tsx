import { memo } from "react"
import { Handle, Position, type Node, type NodeProps } from "@xyflow/react"
import { ChevronDown, ChevronRight } from "lucide-react"

import { usePeek } from "@/features/peek/model/peek-store"
import { cn } from "@/lib/utils"
import type { Member } from "@/shared/bindings/Member"
import type { SymbolKind } from "@/shared/bindings/SymbolKind"
import type { SymbolNode as Symbol } from "@/shared/bindings/SymbolNode"

import { SIZES, splitMembers } from "../../layout/sizes"
import { useDiagram } from "../../model/diagram-store"

export type SymbolFlowNode = Node<{ symbol: Symbol; open: boolean }, "symbol">

const KIND_STYLE: Record<SymbolKind, { letter: string; className: string; label: string }> = {
  class: { letter: "C", className: "bg-blue-500/15 text-blue-300", label: "class" },
  interface: { letter: "I", className: "bg-emerald-500/15 text-emerald-300", label: "interface" },
  enum: { letter: "E", className: "bg-amber-500/15 text-amber-300", label: "enum" },
  typeAlias: { letter: "T", className: "bg-violet-500/15 text-violet-300", label: "type" },
  function: { letter: "ƒ", className: "bg-pink-500/15 text-pink-300", label: "function" },
}

const VISIBILITY = { public: "+", protected: "#", private: "-" } as const

function MemberRow({ member, onPeek }: { member: Member; onPeek: () => void }) {
  return (
    <button
      type="button"
      onClick={onPeek}
      title={member.signature}
      style={{ height: SIZES.rowHeight }}
      className="nodrag flex w-full items-center gap-2 px-3 text-left font-mono text-[11px] text-muted-foreground hover:bg-accent hover:text-accent-foreground"
    >
      <span className="w-2 shrink-0 text-center opacity-70">{VISIBILITY[member.visibility]}</span>
      <span className={cn("truncate", member.isStatic && "underline underline-offset-2", member.isAbstract && "italic")}>
        {member.isAsync && <span className="text-muted-foreground/60">async </span>}
        {member.signature}
      </span>
    </button>
  )
}

function SymbolNodeView({ data, selected }: NodeProps<SymbolFlowNode>) {
  const { symbol, open } = data
  const { toggleOpen } = useDiagram()
  const peek = usePeek()
  const kind = KIND_STYLE[symbol.kind]
  const { fields, methods } = splitMembers(symbol.members)
  const hasMembers = symbol.members.length > 0
  const peekAt = (line: number, title: string) => peek.open({ file: symbol.file, line, title })

  return (
    <div
      className={cn(
        "h-full w-full overflow-hidden rounded-lg border bg-card text-card-foreground shadow-sm transition-shadow",
        symbol.kind === "interface" && "border-dashed",
        selected && "ring-2 ring-ring",
      )}
    >
      <Handle type="source" position={Position.Top} className="!opacity-0" />
      <Handle type="target" position={Position.Bottom} className="!opacity-0" />
      <div className="flex items-center gap-2 px-3" style={{ height: SIZES.headerHeight }}>
        <span
          title={kind.label}
          className={cn("flex size-5 shrink-0 items-center justify-center rounded text-[11px] font-bold", kind.className)}
        >
          {kind.letter}
        </span>
        <button
          type="button"
          className="nodrag min-w-0 flex-1 truncate text-left text-sm font-medium hover:underline"
          onClick={() => peekAt(symbol.range.startLine, symbol.name)}
          title={`${symbol.signature}\n${symbol.file}:${symbol.range.startLine}`}
        >
          <span className={cn(symbol.isAbstract && "italic")}>{symbol.name}</span>
          {symbol.typeParams && <span className="font-mono text-xs text-muted-foreground">{symbol.typeParams}</span>}
        </button>
        {hasMembers && (
          <button
            type="button"
            aria-label={open ? "Hide members" : "Show members"}
            onClick={() => toggleOpen(symbol.id)}
            className="nodrag flex shrink-0 items-center gap-0.5 rounded px-1 text-xs text-muted-foreground hover:bg-accent hover:text-accent-foreground"
          >
            {symbol.members.length}
            {open ? <ChevronDown className="size-3.5" /> : <ChevronRight className="size-3.5" />}
          </button>
        )}
      </div>
      {open && hasMembers && (
        <div className="border-t">
          {fields.length > 0 && (
            <div style={{ paddingBlock: SIZES.sectionGap / 2 }}>
              {fields.map((m) => (
                <MemberRow key={`${m.name}:${m.line}`} member={m} onPeek={() => peekAt(m.line, `${symbol.name}.${m.name}`)} />
              ))}
            </div>
          )}
          {fields.length > 0 && methods.length > 0 && <div className="border-t" />}
          {methods.length > 0 && (
            <div style={{ paddingBlock: SIZES.sectionGap / 2 }}>
              {methods.map((m) => (
                <MemberRow key={`${m.name}:${m.line}`} member={m} onPeek={() => peekAt(m.line, `${symbol.name}.${m.name}`)} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export const SymbolNodeComponent = memo(SymbolNodeView)
