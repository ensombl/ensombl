import { FolderPlus, GitBranch, Loader2 } from "lucide-react"

import { Button } from "@/components/ui/button"
import { useAddProjectFlow } from "@/features/workspaces/model/use-add-project"
import { PaneEmpty } from "@/shared/ui/pane"

export function Welcome() {
  const addProject = useAddProjectFlow()

  return (
    <PaneEmpty
      icon={<GitBranch />}
      title="Add a project to get started"
      description="Pick any folder inside a git repository. Each worktree opens as its own tab with its own UML and agents."
    >
      <Button size="sm" onClick={addProject.run} disabled={addProject.pending}>
        {addProject.pending ? <Loader2 className="animate-spin" /> : <FolderPlus />}
        Add project
      </Button>
    </PaneEmpty>
  )
}
