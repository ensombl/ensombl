import { useMemo, useState } from "react"
import { DoorOpen, Loader2, Route } from "lucide-react"

import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { cn } from "@/lib/utils"
import type { SystemView } from "@/shared/bindings/SystemView"
import { PaneEmpty } from "@/shared/ui/pane"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { usePeekStore } from "@/features/peek/model/peek-store"

import { groupEntries, toSequence } from "../../model/sequence"
import { useSystemUi } from "../../model/system-store"
import { useFlow } from "../../queries/system-queries"
import { SequenceDiagram } from "./sequence-diagram"

const DEPTHS = [2, 4, 6, 8, 10]

/** Pick an entry point, see what it sets in motion across components. */
export function FlowsView({ system }: { system: SystemView }) {
  const worktree = useWorkspace()
  const openPeek = usePeekStore((s) => s.open)
  const ui = useSystemUi()
  const { entry: entryId } = ui.state
  const [query, setQuery] = useState("")
  const [depth, setDepth] = useState(6)
  const nodes = useMemo(() => new Map(system.model.nodes.map((n) => [n.id, n])), [system.model.nodes])
  const nameOf = (id: string) => nodes.get(id)?.name ?? id
  const groups = groupEntries(system.model.entryPoints, query, nameOf)
  const entry = system.model.entryPoints.find((e) => e.id === entryId)
  const { data: steps, isFetching } = useFlow(worktree.id, entry?.id ?? null, depth)
  const sequence = useMemo(() => (entry && steps ? toSequence(entry.node, steps) : null), [entry, steps])
  const peek = (file: string, line: number) => openPeek(worktree.id, { file, line, title: file.split("/").pop() ?? file })

  return (
    <div className="flex h-full">
      <aside className="flex w-72 shrink-0 flex-col border-r">
        <div className="border-b p-2">
          <Input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Filter routes, events, jobs…" className="h-8 text-xs" />
        </div>
        <ScrollArea className="min-h-0 flex-1">
          <div className="space-y-3 p-2">
            {groups.map((g) => (
              <div key={g.kind} className="space-y-0.5">
                <h3 className="px-1.5 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                  {g.label} <span className="font-normal">({g.entries.length})</span>
                </h3>
                {g.entries.map((e) => (
                  <button
                    key={e.id}
                    type="button"
                    onClick={() => ui.openEntry(e.id)}
                    className={cn(
                      "flex w-full flex-col rounded-md px-1.5 py-1 text-left hover:bg-accent",
                      e.id === entryId && "bg-accent text-accent-foreground",
                    )}
                  >
                    <span className="truncate font-mono text-xs">{e.label}</span>
                    <span className="truncate text-[11px] text-muted-foreground">
                      {nameOf(e.node)}
                      {e.member && `.${e.member}`}
                    </span>
                  </button>
                ))}
              </div>
            ))}
            {groups.length === 0 && <p className="p-2 text-xs text-muted-foreground">No entry points found.</p>}
          </div>
        </ScrollArea>
      </aside>
      <div className="flex min-w-0 flex-1 flex-col">
        {!entry ? (
          <PaneEmpty icon={<Route />} title="Pick an entry point" description="See the calls, events and requests it sets off, component by component." />
        ) : (
          <>
            <div className="flex h-10 shrink-0 items-center gap-2 border-b px-3">
              <DoorOpen className="size-4 text-muted-foreground" />
              <span className="truncate font-mono text-sm">{entry.label}</span>
              <button type="button" className="truncate text-xs text-muted-foreground hover:underline" onClick={() => peek(entry.site.file, entry.site.line)}>
                {nameOf(entry.node)}
                {entry.member && `.${entry.member}`}
              </button>
              {isFetching && <Loader2 className="size-3.5 animate-spin text-muted-foreground" />}
              <div className="ml-auto flex items-center gap-2 text-xs text-muted-foreground">
                Depth
                <Select value={String(depth)} onValueChange={(v) => setDepth(Number(v))}>
                  <SelectTrigger size="sm" className="h-7 w-16 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {DEPTHS.map((d) => (
                      <SelectItem key={d} value={String(d)} className="text-xs">
                        {d}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="min-h-0 flex-1 overflow-auto p-4">
              {sequence && sequence.rows.length === 0 && (
                <p className="text-sm text-muted-foreground">No calls to other components were found from here.</p>
              )}
              {sequence && sequence.rows.length > 0 && <SequenceDiagram sequence={sequence} nodes={nodes} onStep={peek} />}
            </div>
          </>
        )}
      </div>
    </div>
  )
}
