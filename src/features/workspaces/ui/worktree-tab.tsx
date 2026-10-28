import { revealItemInDir } from "@tauri-apps/plugin-opener"
import { Copy, FolderOpen, GitBranch, Home, Trash2, X } from "lucide-react"

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu"
import { cn } from "@/lib/utils"
import type { TabView } from "@/shared/bindings/TabView"

import { useActivateTab, useCloseTab } from "../commands/workspace-commands"
import { useWorkspaceDialogs } from "../model/dialogs-store"

interface WorktreeTabProps {
  tab: TabView
  active: boolean
  accent: string
}

export function WorktreeTab({ tab, active, accent }: WorktreeTabProps) {
  const activate = useActivateTab()
  const closeTab = useCloseTab()
  const openRemove = useWorkspaceDialogs((s) => s.openRemoveWorktree)
  const { worktree } = tab
  const close = () => closeTab.mutate({ worktreeId: worktree.id })

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div
          role="tab"
          aria-selected={active}
          title={worktree.path}
          onMouseDown={(e) => {
            if (e.button === 1) close()
          }}
          onClick={() => !active && activate.mutate({ worktreeId: worktree.id })}
          className={cn(
            "group relative flex h-6 max-w-44 shrink-0 cursor-default items-center gap-1 rounded pr-0.5 pl-2 text-xs transition-colors",
            active
              ? "bg-accent text-accent-foreground"
              : "text-muted-foreground hover:bg-accent/50 hover:text-foreground",
          )}
        >
          {worktree.isMain ? <Home className="size-3 shrink-0" /> : <GitBranch className="size-3 shrink-0" />}
          <span className="truncate">{tab.label}</span>
          <button
            type="button"
            aria-label={`Close ${tab.label}`}
            onClick={(e) => {
              e.stopPropagation()
              close()
            }}
            className={cn(
              "flex size-4 shrink-0 items-center justify-center rounded-sm hover:bg-muted-foreground/20",
              active ? "opacity-100" : "opacity-0 group-hover:opacity-100",
            )}
          >
            <X className="size-3" />
          </button>
          <span className={cn("absolute inset-x-2 -bottom-[5px] h-0.5 rounded-full", accent)} />
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onSelect={close}>
          <X /> Close tab
        </ContextMenuItem>
        <ContextMenuItem onSelect={() => revealItemInDir(worktree.path)}>
          <FolderOpen /> Reveal in Finder
        </ContextMenuItem>
        <ContextMenuItem onSelect={() => navigator.clipboard.writeText(worktree.path)}>
          <Copy /> Copy path
        </ContextMenuItem>
        {!worktree.isMain && (
          <>
            <ContextMenuSeparator />
            <ContextMenuItem
              variant="destructive"
              onSelect={() =>
                openRemove({ projectId: worktree.projectId, path: worktree.path, label: tab.label })
              }
            >
              <Trash2 /> Delete worktree…
            </ContextMenuItem>
          </>
        )}
      </ContextMenuContent>
    </ContextMenu>
  )
}
