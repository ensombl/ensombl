import { Loader2, Network, RefreshCw } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { Pane, PaneEmpty } from "@/shared/ui/pane"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { useWorktreeIndex } from "../model/use-worktree-index"
import { DiagramCanvas } from "./diagram/diagram-canvas"

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
              {summary.files} files · {summary.symbols} symbols · {summary.edges} relations · {summary.millis}ms
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
        <DiagramCanvas model={model} />
      )}
    </Pane>
  )
}
