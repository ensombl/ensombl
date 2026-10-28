import { Boxes, Code2, ListTodo, Loader2, Network, RefreshCw, Route, ShieldCheck, Waypoints } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import type { ArchModel } from "@/shared/bindings/ArchModel"
import { ErrorBoundary } from "@/shared/ui/error-boundary"
import { Pane, PaneEmpty } from "@/shared/ui/pane"
import { useIsActiveWorkspace, useWorkspace } from "@/shared/workspace/workspace-context"

import { useDiagram } from "../model/diagram-store"
import { useSystemUi, type ArchitectureTab } from "../model/system-store"
import { useWorktreeIndex } from "../model/use-worktree-index"
import { ancestorsOf } from "../model/view"
import { useSystem } from "../queries/system-queries"
import { ComponentsCanvas } from "./components/components-canvas"
import { BoundariesView } from "./boundaries/boundaries-view"
import { DiagramCanvas } from "./diagram/diagram-canvas"
import { CodeExplorer } from "./code/code-explorer"
import { TasksView } from "@/features/orchestration/ui/tasks-view"
import { FlowsView } from "./flows/flows-view"

const TABS: { value: ArchitectureTab; label: string; icon: typeof Network }[] = [
  { value: "components", label: "Components", icon: Waypoints },
  { value: "flows", label: "Flows", icon: Route },
  { value: "boundaries", label: "Boundaries", icon: ShieldCheck },
  { value: "classes", label: "Classes", icon: Boxes },
  { value: "code", label: "Code", icon: Code2 },
  { value: "tasks", label: "Tasks", icon: ListTodo },
]

function Lenses({ model, worktreeId }: { model: ArchModel; worktreeId: string }) {
  const ui = useSystemUi()
  const active = useIsActiveWorkspace()
  const diagram = useDiagram()
  const { data: system } = useSystem(worktreeId)
  if (!active) return null

  const showClasses = (symbolId: string) => {
    const kind = model.symbols.find((s) => s.id === symbolId)?.kind
    diagram.reveal(symbolId, ancestorsOf(model, symbolId), kind === "function" || kind === "typeAlias")
    ui.setTab("classes")
  }

  return (
    <Tabs value={ui.state.tab} onValueChange={(v) => ui.setTab(v as ArchitectureTab)} className="flex h-full flex-col gap-0">
      {/*
        Only the visible lens is mounted (Radix unmounts the rest), and none in a
        background worktree: hidden maps kept re-laying out on every file change,
        queueing on the one ELK worker ahead of the map you're looking at.
      */}
      <TabsContent value="components" className="min-h-0 flex-1">
        {system ? (
          <ErrorBoundary resetKey={system}>
            <ComponentsCanvas system={system} onShowClasses={showClasses} />
          </ErrorBoundary>
        ) : (
          <PaneEmpty icon={<Loader2 className="animate-spin" />} title="Working out the architecture…" description="Finding components and how they talk" />
        )}
      </TabsContent>
      <TabsContent value="flows" className="min-h-0 flex-1">
        {system && (
          <ErrorBoundary resetKey={system}>
            <FlowsView system={system} />
          </ErrorBoundary>
        )}
      </TabsContent>
      <TabsContent value="boundaries" className="min-h-0 flex-1">
        {system && (
          <ErrorBoundary resetKey={system}>
            <BoundariesView system={system} />
          </ErrorBoundary>
        )}
      </TabsContent>
      <TabsContent value="classes" className="min-h-0 flex-1">
        <ErrorBoundary resetKey={model}>
          <DiagramCanvas model={model} />
        </ErrorBoundary>
      </TabsContent>
      <TabsContent value="code" className="min-h-0 flex-1">
        <ErrorBoundary resetKey={worktreeId}>
          <CodeExplorer />
        </ErrorBoundary>
      </TabsContent>
      <TabsContent value="tasks" className="min-h-0 flex-1">
        <ErrorBoundary resetKey={worktreeId}>
          <TasksView />
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
