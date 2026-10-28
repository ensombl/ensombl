import { useMemo } from "react"
import { Loader2, Network, RefreshCw } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Progress } from "@/components/ui/progress"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Pane, PaneEmpty } from "@/shared/ui/pane"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { useWorktreeIndex } from "../model/use-worktree-index"

export function ArchitecturePane() {
  const worktree = useWorkspace()
  const { model, summary, indexing, progress, reindex } = useWorktreeIndex(worktree.id)

  const packages = useMemo(() => {
    const counts = new Map<string, number>()
    for (const s of model?.symbols ?? []) counts.set(s.package, (counts.get(s.package) ?? 0) + 1)
    return [...counts.entries()].sort((a, b) => b[1] - a[1])
  }, [model])

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
        <ScrollArea className="h-full">
          <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3 p-4">
            {packages.map(([name, count]) => (
              <Card key={name} className="gap-2 py-4">
                <CardHeader className="px-4">
                  <CardTitle className="truncate font-mono text-sm">{name}</CardTitle>
                </CardHeader>
                <CardContent className="px-4">
                  <Badge variant="secondary">{count} symbols</Badge>
                </CardContent>
              </Card>
            ))}
          </div>
        </ScrollArea>
      )}
    </Pane>
  )
}
