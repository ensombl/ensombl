import { useMemo, useState } from "react"
import { ChevronRight, DoorOpen, Loader2, PanelLeftClose, PanelLeftOpen, Route } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"

import { Input } from "@/components/ui/input"
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable"
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

const VERB_COLOR: Record<string, string> = {
  GET: "text-sky-300",
  POST: "text-emerald-300",
  PUT: "text-amber-300",
  PATCH: "text-amber-300",
  DELETE: "text-red-300",
}

/** `POST /claims/:id` → verb and path; other labels have no verb. */
function splitLabel(label: string): { verb: string | null; rest: string } {
  const match = /^([A-Z]+) (\/.*)$/.exec(label)
  return match ? { verb: match[1], rest: match[2] } : { verb: null, rest: label }
}

/** Pick an entry point, see what it sets in motion across components. */
export function FlowsView({ system }: { system: SystemView }) {
  const worktree = useWorkspace()
  const openPeek = usePeekStore((s) => s.open)
  const ui = useSystemUi()
  const { entry: entryId } = ui.state
  const [query, setQuery] = useState("")
  const [depth, setDepth] = useState(6)
  const [listOpen, setListOpen] = useState(true)
  const [closedGroups, setClosedGroups] = useState<Set<string>>(new Set())
  const toggleGroup = (kind: string, open: boolean) =>
    setClosedGroups((prev) => {
      const next = new Set(prev)
      if (open) next.delete(kind)
      else next.add(kind)
      return next
    })
  const nodes = useMemo(() => new Map(system.model.nodes.map((n) => [n.id, n])), [system.model.nodes])
  const nameOf = (id: string) => nodes.get(id)?.name ?? id
  const groups = groupEntries(system.model.entryPoints, query, nameOf)
  const entry = system.model.entryPoints.find((e) => e.id === entryId)
  const { data: steps, isFetching } = useFlow(worktree.id, entry?.id ?? null, depth)
  const sequence = useMemo(() => (entry && steps ? toSequence(entry.node, steps) : null), [entry, steps])
  const peek = (file: string, line: number) => openPeek(worktree.id, { file, line, title: file.split("/").pop() ?? file })

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full">
      {listOpen && (
        <>
          <ResizablePanel id="entries" defaultSize="34" minSize="20" maxSize="70" className="flex flex-col">
            <div className="flex items-center gap-1 border-b p-2">
              <Input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Filter routes, events, jobs…" className="h-8 text-xs" />
              <Button variant="ghost" size="icon" className="size-8 shrink-0" onClick={() => setListOpen(false)} aria-label="Hide entry points">
                <PanelLeftClose />
              </Button>
            </div>
            <ScrollArea className="min-h-0 flex-1">
              <div className="space-y-3 p-2">
                {groups.map((g) => (
                  <Collapsible
                    key={g.kind}
                    open={!closedGroups.has(g.kind)}
                    onOpenChange={(open) => toggleGroup(g.kind, open)}
                    className="space-y-0.5"
                  >
                    <CollapsibleTrigger className="group flex w-full items-center gap-1 px-1.5 text-[11px] font-medium tracking-wide text-muted-foreground uppercase hover:text-foreground">
                      <ChevronRight className="size-3 transition-transform group-data-[state=open]:rotate-90" />
                      {g.label} <span className="font-normal">({g.entries.length})</span>
                    </CollapsibleTrigger>
                    <CollapsibleContent className="space-y-0.5">
                    {g.entries.map((e) => {
                      const { verb, rest } = splitLabel(e.label)
                      return (
                        <button
                          key={e.id}
                          type="button"
                          onClick={() => ui.openEntry(e.id)}
                          title={`${e.label}\n${e.site.file}:${e.site.line}`}
                          className={cn(
                            "flex w-full gap-2 rounded-md px-1.5 py-1.5 text-left hover:bg-accent",
                            e.id === entryId && "bg-accent text-accent-foreground",
                          )}
                        >
                          {verb && <span className={cn("w-12 shrink-0 pt-px font-mono text-[10px] font-semibold", VERB_COLOR[verb])}>{verb}</span>}
                          <span className="min-w-0 space-y-0.5">
                            <span className="block font-mono text-xs break-all">{rest}</span>
                            <span className="block truncate text-[11px] text-muted-foreground">
                              {nameOf(e.node)}
                              {e.member && `.${e.member}`}
                            </span>
                          </span>
                        </button>
                      )
                    })}
                    </CollapsibleContent>
                  </Collapsible>
                ))}
                {groups.length === 0 && <p className="p-2 text-xs text-muted-foreground">No entry points found.</p>}
              </div>
            </ScrollArea>
          </ResizablePanel>
          <ResizableHandle />
        </>
      )}
      <ResizablePanel id="flow" minSize="30" className="relative flex min-w-0 flex-col">
        {!listOpen && (
          <Button variant="ghost" size="sm" className="absolute top-1.5 left-2 z-20 h-7 gap-1.5 text-xs" onClick={() => setListOpen(true)}>
            <PanelLeftOpen /> Entry points
          </Button>
        )}
        {!entry ? (
          <PaneEmpty icon={<Route />} title="Pick an entry point" description="See the calls, events and requests it sets off, component by component." />
        ) : (
          <>
            <div className={cn("flex h-10 shrink-0 items-center gap-2 border-b px-3", !listOpen && "pl-36")}>
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
            <div className="min-h-0 flex-1 overflow-auto px-4 pb-4">
              {sequence && sequence.rows.length === 0 && (
                <p className="text-sm text-muted-foreground">No calls to other components were found from here.</p>
              )}
              {sequence && sequence.rows.length > 0 && <SequenceDiagram sequence={sequence} nodes={nodes} onStep={peek} />}
            </div>
          </>
        )}
      </ResizablePanel>
    </ResizablePanelGroup>
  )
}
