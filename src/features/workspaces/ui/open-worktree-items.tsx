import { GitBranch, Home, Loader2 } from "lucide-react"

import { DropdownMenuItem } from "@/components/ui/dropdown-menu"

import { useOpenWorktree } from "../commands/workspace-commands"
import { useWorktrees } from "../queries/workspace-queries"

/** Menu items for the project's worktrees that aren't open in a tab yet. */
export function OpenWorktreeItems({ projectId }: { projectId: string }) {
  const { data, isLoading } = useWorktrees(projectId)
  const openWorktree = useOpenWorktree()
  const closed = data?.filter((w) => !w.isOpen) ?? []

  if (isLoading) {
    return (
      <DropdownMenuItem disabled>
        <Loader2 className="animate-spin" /> Loading…
      </DropdownMenuItem>
    )
  }
  if (closed.length === 0) {
    return <DropdownMenuItem disabled>All worktrees are open</DropdownMenuItem>
  }
  return closed.map(({ worktree, label }) => (
    <DropdownMenuItem
      key={worktree.id}
      title={worktree.path}
      onSelect={() => openWorktree.mutate({ projectId, path: worktree.path })}
    >
      {worktree.isMain ? <Home /> : <GitBranch />}
      {label}
    </DropdownMenuItem>
  ))
}
