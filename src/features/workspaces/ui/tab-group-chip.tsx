import { ChevronRight, FolderGit2, GitBranchPlus, Trash2 } from "lucide-react"

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu"
import { cn } from "@/lib/utils"
import type { Project } from "@/shared/bindings/Project"

import { useRemoveProject, useToggleGroup } from "../commands/workspace-commands"
import { useWorkspaceDialogs } from "../model/dialogs-store"
import { groupColors } from "../model/group-colors"

export function TabGroupChip({ project, tabCount }: { project: Project; tabCount: number }) {
  const toggle = useToggleGroup()
  const removeProject = useRemoveProject()
  const openCreate = useWorkspaceDialogs((s) => s.openCreateWorktree)
  const colors = groupColors[project.color]

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <button
          type="button"
          title={project.root}
          onClick={() => toggle.mutate({ projectId: project.id })}
          className={cn(
            "flex h-6 shrink-0 items-center gap-1 rounded-md px-2 text-[11px] font-semibold transition-colors",
            colors.chip,
          )}
        >
          <span className="max-w-32 truncate">{project.name}</span>
          {project.collapsed && (
            <>
              <span className="opacity-70">{tabCount}</span>
              <ChevronRight className="size-3 opacity-70" />
            </>
          )}
        </button>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onSelect={() => openCreate(project.id)}>
          <GitBranchPlus /> New worktree…
        </ContextMenuItem>
        <ContextMenuItem onSelect={() => toggle.mutate({ projectId: project.id })}>
          <FolderGit2 /> {project.collapsed ? "Expand group" : "Collapse group"}
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem variant="destructive" onSelect={() => removeProject.mutate({ projectId: project.id })}>
          <Trash2 /> Remove project from Ensombl
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}
