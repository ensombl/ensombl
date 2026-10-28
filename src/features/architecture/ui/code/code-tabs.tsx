import { MessageSquare, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { draftKey, useDraftsStore } from "../../model/drafts-store"
import { useSystemUi } from "../../model/system-store"

/** Open files as tabs; a dot marks unsaved edits. */
export function CodeTabs({ review, onReview }: { review: boolean; onReview: (review: boolean) => void }) {
  const worktree = useWorkspace()
  const ui = useSystemUi()
  const { openFiles, file: active } = ui.state
  const dirty = useDraftsStore((s) => s.dirty)

  return (
    <div className="flex h-10 shrink-0 items-stretch border-b">
      <div role="tablist" className="flex min-w-0 flex-1 overflow-x-auto overflow-y-hidden [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
        {openFiles.map((path) => {
          const unsaved = !!dirty[draftKey(worktree.id, path)]
          return (
            <div
              key={path}
              role="tab"
              aria-selected={path === active}
              title={path}
              onClick={() => ui.openFile(path)}
              onAuxClick={(e) => e.button === 1 && ui.closeFile(path)}
              className={cn(
                "group flex max-w-52 shrink-0 cursor-default items-center gap-1.5 border-r pr-1.5 pl-3 text-xs text-muted-foreground hover:bg-accent/50",
                path === active && "bg-[#0d1117] text-foreground",
              )}
            >
              <span className="truncate">{path.slice(path.lastIndexOf("/") + 1)}</span>
              <button
                type="button"
                aria-label={`Close ${path}`}
                onClick={(e) => {
                  e.stopPropagation()
                  ui.closeFile(path)
                }}
                className="group/close flex size-4 shrink-0 items-center justify-center rounded-sm hover:bg-accent"
              >
                {unsaved ? (
                  <>
                    <span className="size-1.5 rounded-full bg-foreground group-hover/close:hidden" />
                    <X className="hidden size-3 group-hover/close:block" />
                  </>
                ) : (
                  <X className={cn("size-3 opacity-0 group-hover:opacity-100", path === active && "opacity-100")} />
                )}
              </button>
            </div>
          )
        })}
      </div>
      {active && (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant={review ? "secondary" : "ghost"}
              size="icon"
              className="m-1.5 size-7 shrink-0"
              aria-label="Comment on lines"
              onClick={() => onReview(!review)}
            >
              <MessageSquare />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{review ? "Back to editing" : "Comment on lines"}</TooltipContent>
        </Tooltip>
      )}
    </div>
  )
}
