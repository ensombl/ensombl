import { FolderPlus, GitBranchPlus, FolderGit2, Plus } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { cn } from "@/lib/utils"
import type { Project } from "@/shared/bindings/Project"

import { useWorkspaceDialogs } from "../model/dialogs-store"
import { groupColors } from "../model/group-colors"
import { useAddProjectFlow } from "../model/use-add-project"
import { OpenWorktreeItems } from "./open-worktree-items"

export function NewTabMenu({ projects }: { projects: Project[] }) {
  const openCreate = useWorkspaceDialogs((s) => s.openCreateWorktree)
  const addProject = useAddProjectFlow()

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="size-7 shrink-0" aria-label="New tab">
          <Plus />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-64">
        {projects.length > 0 && <DropdownMenuLabel>Projects</DropdownMenuLabel>}
        {projects.map((project) => (
          <DropdownMenuSub key={project.id}>
            <DropdownMenuSubTrigger>
              <span className={cn("size-2 rounded-full", groupColors[project.color].dot)} />
              {project.name}
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent className="w-56">
              <DropdownMenuItem onSelect={() => openCreate(project.id)}>
                <GitBranchPlus /> New worktree…
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuLabel className="flex items-center gap-2">
                <FolderGit2 className="size-3.5" /> Open worktree
              </DropdownMenuLabel>
              <OpenWorktreeItems projectId={project.id} />
            </DropdownMenuSubContent>
          </DropdownMenuSub>
        ))}
        {projects.length > 0 && <DropdownMenuSeparator />}
        <DropdownMenuItem onSelect={addProject.run} disabled={addProject.pending}>
          <FolderPlus /> Add project…
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
