import { useMemo, type ReactNode } from "react"
import { FileCode2, X } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Separator } from "@/components/ui/separator"
import { usePeek } from "@/features/peek/model/peek-store"
import { cn } from "@/lib/utils"
import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { EdgeKind } from "@/shared/bindings/EdgeKind"
import type { Member } from "@/shared/bindings/Member"
import type { SymbolNode } from "@/shared/bindings/SymbolNode"

import { splitMembers } from "../../layout/sizes"
import { KindBadge, kindLabel } from "../kind-badge"

const VISIBILITY = { public: "+", protected: "#", private: "-" } as const

const RELATIONS: { kind: EdgeKind; label: string }[] = [
  { kind: "extends", label: "Extends" },
  { kind: "implements", label: "Implements" },
  { kind: "association", label: "Has" },
  { kind: "dependency", label: "Uses" },
]

interface SymbolInspectorProps {
  model: ArchModel
  symbol: SymbolNode
  onNavigate: (symbolId: string) => void
  onClose: () => void
}

function Section({ title, count, children }: { title: string; count?: number; children: ReactNode }) {
  return (
    <section className="space-y-1.5 px-4 py-3">
      <h3 className="flex items-center gap-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
        {title}
        {count !== undefined && <span className="font-mono normal-case">{count}</span>}
      </h3>
      {children}
    </section>
  )
}

function MemberRow({ member, onPeek }: { member: Member; onPeek: () => void }) {
  return (
    <div className="group flex items-start rounded-md hover:bg-accent hover:text-accent-foreground">
      <button
        type="button"
        onClick={onPeek}
        title={`${member.signature} (line ${member.line})`}
        className="flex min-w-0 flex-1 items-start gap-2 px-2 py-1 text-left font-mono text-xs"
      >
        <span className="w-2 shrink-0 text-muted-foreground">{VISIBILITY[member.visibility]}</span>
        <span className={cn("min-w-0 break-words", member.isStatic && "underline underline-offset-2", member.isAbstract && "italic")}>
          {member.isAsync && <span className="text-muted-foreground">async </span>}
          {member.signature}
        </span>
      </button>
    </div>
  )
}

function SymbolLink({ symbol, onClick }: { symbol: SymbolNode; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={symbol.file}
      className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-accent hover:text-accent-foreground"
    >
      <KindBadge kind={symbol.kind} className="size-4 text-[9px]" />
      <span className="truncate">{symbol.name}</span>
      <span className="ml-auto truncate pl-2 text-xs text-muted-foreground">{symbol.package}</span>
    </button>
  )
}

/** Details for the selected symbol: members with signatures and its relations. */
export function SymbolInspector({ model, symbol, onNavigate, onClose }: SymbolInspectorProps) {
  const peek = usePeek()
  const { fields, methods } = splitMembers(symbol.members)
  const peekAt = (line: number, title: string) => peek.open({ file: symbol.file, line, title })

  const relations = useMemo(() => {
    const byId = new Map(model.symbols.map((s) => [s.id, s]))
    const outgoing = new Map<EdgeKind, SymbolNode[]>()
    const incoming: SymbolNode[] = []
    for (const e of model.edges) {
      if (e.from === symbol.id) {
        const target = byId.get(e.to)
        if (target) outgoing.set(e.kind, [...(outgoing.get(e.kind) ?? []), target])
      } else if (e.to === symbol.id) {
        const source = byId.get(e.from)
        if (source && !incoming.includes(source)) incoming.push(source)
      }
    }
    return { outgoing, incoming }
  }, [model, symbol.id])

  return (
    <aside className="absolute inset-x-3 bottom-3 z-20 flex max-h-[48%] flex-col overflow-hidden rounded-xl border bg-popover text-popover-foreground shadow-lg">
      <header className="flex items-start gap-2 border-b p-4">
        <KindBadge kind={symbol.kind} className="mt-0.5" />
        <div className="min-w-0 flex-1">
          <h2 className={cn("truncate font-semibold", symbol.isAbstract && "italic")}>
            {symbol.name}
            {symbol.typeParams && <span className="font-mono text-sm font-normal text-muted-foreground">{symbol.typeParams}</span>}
          </h2>
          <div className="mt-1 flex flex-wrap items-center gap-1.5">
            <Badge variant="secondary">{kindLabel(symbol.kind)}</Badge>
            {symbol.isAbstract && <Badge variant="outline">abstract</Badge>}
            {!symbol.exported && <Badge variant="outline">internal</Badge>}
            <Badge variant="outline" className="max-w-full truncate">
              {symbol.package}
            </Badge>
          </div>
        </div>
        <Button variant="ghost" size="icon" className="size-7 shrink-0" onClick={onClose} aria-label="Close inspector">
          <X />
        </Button>
      </header>
      {/* A plain overflow box: a ScrollArea can't size itself inside a max-height panel. */}
      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="px-4 pt-3">
          <Button
            variant="outline"
            size="sm"
            className="w-full justify-start font-mono text-xs"
            onClick={() => peekAt(symbol.range.startLine, symbol.name)}
          >
            <FileCode2 />
            <span className="truncate">
              {symbol.file}:{symbol.range.startLine}
            </span>
          </Button>
          <p className="mt-2 font-mono text-xs break-words text-muted-foreground">{symbol.signature}</p>
        </div>
        {fields.length > 0 && (
          <Section title={symbol.kind === "enum" ? "Members" : "Fields"} count={fields.length}>
            {fields.map((m) => (
              <MemberRow key={`${m.name}:${m.line}`} member={m} onPeek={() => peekAt(m.line, `${symbol.name}.${m.name}`)} />
            ))}
          </Section>
        )}
        {methods.length > 0 && (
          <Section title="Methods" count={methods.length}>
            {methods.map((m) => (
              <MemberRow key={`${m.name}:${m.line}`} member={m} onPeek={() => peekAt(m.line, `${symbol.name}.${m.name}`)} />
            ))}
          </Section>
        )}
        <Separator />
        {RELATIONS.map(({ kind, label }) => {
          const targets = relations.outgoing.get(kind) ?? []
          if (targets.length === 0) return null
          return (
            <Section key={kind} title={label} count={targets.length}>
              {targets.map((t) => (
                <SymbolLink key={t.id} symbol={t} onClick={() => onNavigate(t.id)} />
              ))}
            </Section>
          )
        })}
        {relations.incoming.length > 0 && (
          <Section title="Used by" count={relations.incoming.length}>
            {relations.incoming.map((s) => (
              <SymbolLink key={s.id} symbol={s} onClick={() => onNavigate(s.id)} />
            ))}
          </Section>
        )}
      </div>
    </aside>
  )
}
