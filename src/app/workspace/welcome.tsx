import { FolderPlus, Loader2 } from "lucide-react"

import logo from "@/assets/logo.png"

import { Button } from "@/components/ui/button"
import { useAddProjectFlow } from "@/features/workspaces/model/use-add-project"

export function Welcome() {
  const addProject = useAddProjectFlow()

  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 p-6 text-center">
      <img src={logo} alt="CRM" className="size-24" draggable={false} />
      <div className="space-y-1">
        <p className="text-sm font-medium">Add a project to get started</p>
        <p className="max-w-xs text-sm text-muted-foreground">
          Pick any folder inside a git repository. Each worktree opens as its own tab with its own architecture map and agents.
        </p>
      </div>
      <Button size="sm" onClick={addProject.run} disabled={addProject.pending}>
        {addProject.pending ? <Loader2 className="animate-spin" /> : <FolderPlus />}
        Add project
      </Button>
    </div>
  )
}
