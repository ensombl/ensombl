import { useMemo } from "react"
import { Check, Loader2, Shapes, Undo2, X } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { useDiagram } from "@/features/architecture/model/diagram-store"
import { ancestorsOf } from "@/features/architecture/model/view"
import { cn } from "@/lib/utils"
import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { PatternKind } from "@/shared/bindings/PatternKind"
import type { PatternView } from "@/shared/bindings/PatternView"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { useSetPatternDecision } from "../commands/pattern-commands"
import { usePatterns } from "../queries/pattern-queries"

const KIND_LABEL: Record<PatternKind, string> = {
  strategy: "Strategy",
  factory: "Factory",
  builder: "Builder",
  observer: "Observer",
  singleton: "Singleton",
  decorator: "Decorator",
  adapter: "Adapter",
  repository: "Repository",
  facade: "Facade",
}

function PatternRow({ view, active, onSpotlight }: { view: PatternView; active: boolean; onSpotlight: () => void }) {
  const worktree = useWorkspace()
  const decide = useSetPatternDecision()
  const { pattern, decision } = view
  const set = (d: PatternView["decision"]) =>
    decide.mutate({ worktreeId: worktree.id, patternId: pattern.id, decision: d })

  return (
    <div
      className={cn(
        "group rounded-md border border-transparent px-2 py-1.5 hover:bg-accent/60",
        active && "border-violet-500/50 bg-violet-500/10",
        decision === "dismissed" && "opacity-50",
      )}
    >
      <button type="button" onClick={onSpotlight} className="flex w-full items-center gap-2 text-left">
        <span className="min-w-0 flex-1 truncate text-sm">{pattern.name.split(": ")[1] ?? pattern.name}</span>
        {decision === "confirmed" && <Check className="size-3.5 text-emerald-400" />}
        <span className="font-mono text-[10px] text-muted-foreground">{Math.round(pattern.confidence * 100)}%</span>
      </button>
      <p className="truncate text-xs text-muted-foreground" title={pattern.evidence.join("; ")}>
        {pattern.participants.length} participants · {pattern.evidence[0]}
      </p>
      <div className="mt-1 flex gap-1 opacity-0 group-hover:opacity-100">
        {decision ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button size="icon" variant="ghost" className="size-6" onClick={() => set(null)}>
                <Undo2 className="size-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Undo decision</TooltipContent>
          </Tooltip>
        ) : (
          <>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button size="icon" variant="ghost" className="size-6" onClick={() => set("confirmed")}>
                  <Check className="size-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Confirm</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button size="icon" variant="ghost" className="size-6" onClick={() => set("dismissed")}>
                  <X className="size-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Dismiss (not a real instance)</TooltipContent>
            </Tooltip>
          </>
        )}
      </div>
    </div>
  )
}

/** Detected design patterns, grouped by kind. Clicking one spotlights its participants. */
export function PatternsPanel({ model }: { model: ArchModel }) {
  const worktree = useWorkspace()
  const { data, isLoading } = usePatterns(worktree.id)
  const diagram = useDiagram()

  const groups = useMemo(() => {
    const byKind = new Map<PatternKind, PatternView[]>()
    for (const view of data ?? []) byKind.set(view.pattern.kind, [...(byKind.get(view.pattern.kind) ?? []), view])
    return [...byKind.entries()].sort((a, b) => b[1].length - a[1].length)
  }, [data])

  const spotlight = (view: PatternView) => {
    const roles: Record<string, string> = {}
    for (const p of view.pattern.participants) roles[p.symbolId] ??= p.role
    const ancestors = Object.keys(roles).flatMap((id) => ancestorsOf(model, id))
    diagram.spotlight(view.pattern.id, roles, ancestors)
  }

  return (
    <aside className="flex h-full w-72 shrink-0 flex-col border-r bg-background">
      <header className="flex h-10 items-center gap-2 border-b px-3">
        <Shapes className="size-4 text-violet-300" />
        <h2 className="text-sm font-medium">Patterns</h2>
        {data && <Badge variant="secondary">{data.filter((v) => v.decision !== "dismissed").length}</Badge>}
        <Button variant="ghost" size="icon" className="ml-auto size-7" onClick={diagram.togglePatterns} aria-label="Close patterns">
          <X />
        </Button>
      </header>
      <ScrollArea className="min-h-0 flex-1">
        {isLoading && (
          <div className="flex items-center gap-2 p-4 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" /> Detecting…
          </div>
        )}
        {data?.length === 0 && <p className="p-4 text-sm text-muted-foreground">No patterns detected yet.</p>}
        {groups.map(([kind, views]) => (
          <section key={kind} className="px-2 py-2">
            <h3 className="flex items-center gap-2 px-2 pb-1 text-xs font-medium tracking-wide text-muted-foreground uppercase">
              {KIND_LABEL[kind]} <span className="font-mono normal-case">{views.length}</span>
            </h3>
            {views.map((view) => (
              <PatternRow
                key={view.pattern.id}
                view={view}
                active={diagram.state.spotlight?.patternId === view.pattern.id}
                onSpotlight={() => spotlight(view)}
              />
            ))}
          </section>
        ))}
      </ScrollArea>
    </aside>
  )
}
