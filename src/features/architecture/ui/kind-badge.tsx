import { cn } from "@/lib/utils"
import type { SymbolKind } from "@/shared/bindings/SymbolKind"

const KIND_STYLE: Record<SymbolKind, { letter: string; className: string; label: string }> = {
  class: { letter: "C", className: "bg-blue-500/15 text-blue-300", label: "class" },
  interface: { letter: "I", className: "bg-emerald-500/15 text-emerald-300", label: "interface" },
  enum: { letter: "E", className: "bg-amber-500/15 text-amber-300", label: "enum" },
  typeAlias: { letter: "T", className: "bg-violet-500/15 text-violet-300", label: "type" },
  function: { letter: "ƒ", className: "bg-pink-500/15 text-pink-300", label: "function" },
}

export const kindLabel = (kind: SymbolKind) => KIND_STYLE[kind].label

export function KindBadge({ kind, className }: { kind: SymbolKind; className?: string }) {
  const style = KIND_STYLE[kind]
  return (
    <span
      title={style.label}
      className={cn("flex size-5 shrink-0 items-center justify-center rounded text-[11px] font-bold", style.className, className)}
    >
      {style.letter}
    </span>
  )
}
