import { useState, type FormEvent } from "react"
import { GitBranchPlus, Loader2 } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import type { Project } from "@/shared/bindings/Project"

import { useCreateWorktree } from "../commands/workspace-commands"
import { suggestBranchName } from "../model/branch-names"
import { useWorkspaceDialogs } from "../model/dialogs-store"
import { useBranches } from "../queries/workspace-queries"

const HEAD = "__head__"

export function CreateWorktreeDialog({ projects }: { projects: Project[] }) {
  const projectId = useWorkspaceDialogs((s) => s.createWorktreeFor)
  const close = useWorkspaceDialogs((s) => s.close)
  const project = projects.find((p) => p.id === projectId)

  return (
    <Dialog open={!!project} onOpenChange={(open) => !open && close()}>
      <DialogContent className="sm:max-w-md">
        {project && <CreateWorktreeForm key={project.id} project={project} onDone={close} />}
      </DialogContent>
    </Dialog>
  )
}

function CreateWorktreeForm({ project, onDone }: { project: Project; onDone: () => void }) {
  const branches = useBranches(project.id)
  const create = useCreateWorktree()
  const [branch, setBranch] = useState(() => suggestBranchName([]))
  const [base, setBase] = useState(HEAD)

  const exists = branches.data?.includes(branch.trim()) ?? false

  const submit = (e: FormEvent) => {
    e.preventDefault()
    create.mutate(
      { projectId: project.id, branch: branch.trim(), base: base === HEAD ? undefined : base },
      { onSuccess: onDone },
    )
  }

  return (
    <form onSubmit={submit} className="grid gap-4">
      <DialogHeader>
        <DialogTitle>New worktree in {project.name}</DialogTitle>
        <DialogDescription>
          A separate checkout on its own branch, so agents can work in parallel without stepping on each other.
        </DialogDescription>
      </DialogHeader>
      <div className="grid gap-2">
        <Label htmlFor="branch">Branch</Label>
        <Input
          id="branch"
          autoFocus
          value={branch}
          onChange={(e) => setBranch(e.target.value)}
          className="font-mono"
          spellCheck={false}
        />
        <p className="text-xs text-muted-foreground">
          {exists ? "Existing branch: it will be checked out." : "A new branch will be created."}
        </p>
      </div>
      {!exists && (
        <div className="grid gap-2">
          <Label>Base</Label>
          <Select value={base} onValueChange={setBase}>
            <SelectTrigger className="w-full font-mono">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={HEAD}>Current HEAD of {project.name}</SelectItem>
              {branches.data?.map((b) => (
                <SelectItem key={b} value={b} className="font-mono">
                  {b}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onDone}>
          Cancel
        </Button>
        <Button type="submit" disabled={!branch.trim() || create.isPending}>
          {create.isPending ? <Loader2 className="animate-spin" /> : <GitBranchPlus />}
          Create worktree
        </Button>
      </DialogFooter>
    </form>
  )
}
