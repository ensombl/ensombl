import { Code2, Loader2, Network, RefreshCw, Waypoints } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import type { ArchModel } from "@/shared/bindings/ArchModel"
import { ErrorBoundary } from "@/shared/ui/error-boundary"
import { Pane, PaneEmpty } from "@/shared/ui/pane"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { useDiagram } from "../model/diagram-store"
import { useSystemUi, type ArchitectureTab } from "../model/system-store"
import { useWorktreeIndex } from "../model/use-worktree-index"
import { ancestorsOf } from "../model/view"
import { useSystem } from "../queries/system-queries"
import { ComponentsCanvas } from "./components/components-canvas"
import { DiagramCanvas } from "./diagram/diagram-canvas"

const TABS: { value: ArchitectureTab; label: string; icon: typeof Network }[] = [
  { value: "components", label: "Components", icon: Waypoints },
  { value: "code", label: "Code", icon: Code2 },
]

function Lenses({ model, worktreeId }: { model: ArchModel; worktreeId: string }) {
  const ui = useSystemUi()
  const diagram = useDiagram()
  const { data: system } = useSystem(worktreeId)

  const showCode = (symbolId: string) => {
    const kind = model.symbols.find((s) => s.id === symbolId)?.kind
    diagram.reveal(symbolId, ancestorsOf(model, symbolId), kind === "function" || kind === "typeAlias")
    ui.setTab("code")
  }

  return (
    <Tabs value={ui.state.tab} onValueChange={(v) => ui.setTab(v as ArchitectureTab)} className="flex h-full flex-col gap-0">
      {/* Kept mounted so each lens keeps its viewport when you switch back. */}
      <TabsContent value="components" forceMount className="min-h-0 flex-1 data-[state=inactive]:hidden">
        {system ? (
          <ErrorBoundary resetKey={system}>
            <ComponentsCanvas system={system} onShowCode={showCode} />
          </ErrorBoundary>
        ) : (
          <PaneEmpty icon={<Loader2 className="animate-spin" />} title="Working out the architecture…" description="Finding components and how they talk" />
        )}
      </TabsContent>
      <TabsContent value="code" forceMount className="min-h-0 flex-1 data-[state=inactive]:hidden">
        <ErrorBoundary resetKey={model}>
          <DiagramCanvas model={model} />
        </ErrorBoundary>
      </TabsContent>
      <div className="flex h-10 shrink-0 items-center border-t px-2">
        <TabsList className="h-8">
          {TABS.map(({ value, label, icon: Icon }) => (
            <TabsTrigger key={value} value={value} className="gap-1.5 px-3 text-xs">
              <Icon /> {label}
            </TabsTrigger>
          ))}
        </TabsList>
      </div>
    </Tabs>
  )
}

export function ArchitecturePane() {
  const worktree = useWorkspace()
  const { model, summary, indexing, progress, reindex } = useWorktreeIndex(worktree.id)

  return (
    <Pane
      title="Architecture"
      icon={<Network />}
      actions={
        <>
          {summary && (
            <span className="text-xs text-muted-foreground">
              {summary.files} files · {summary.symbols} symbols · {summary.millis}ms
            </span>
          )}
          <Button variant="ghost" size="icon" className="size-7" onClick={reindex} disabled={indexing} aria-label="Reindex">
            <RefreshCw className={indexing ? "animate-spin" : undefined} />
          </Button>
        </>
      }
    >
      {!model ? (
        <PaneEmpty
          icon={indexing ? <Loader2 className="animate-spin" /> : <Network />}
          title={indexing ? "Indexing worktree…" : "Not indexed"}
          description={worktree.path}
        >
          {progress && (
            <div className="w-64 space-y-1">
              <Progress value={(progress.done / Math.max(progress.total, 1)) * 100} />
              <p className="text-xs text-muted-foreground">
                {progress.done} / {progress.total} files
              </p>
            </div>
          )}
        </PaneEmpty>
      ) : (
        <Lenses model={model} worktreeId={worktree.id} />
      )}
    </Pane>
  )
}
