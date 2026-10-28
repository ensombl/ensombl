import { useState } from "react"
import { Loader2, Trash2 } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"

import { useRemoveWorktree } from "../commands/workspace-commands"
import { useWorkspaceDialogs } from "../model/dialogs-store"

export function RemoveWorktreeDialog() {
  const target = useWorkspaceDialogs((s) => s.removeWorktree)
  const close = useWorkspaceDialogs((s) => s.close)
  const remove = useRemoveWorktree()
  const [force, setForce] = useState(false)

  return (
    <Dialog
      open={!!target}
      onOpenChange={(open) => {
        if (!open) {
          close()
          setForce(false)
        }
      }}
    >
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Delete worktree “{target?.label}”?</DialogTitle>
          <DialogDescription className="break-all">
            Runs <code className="font-mono">git worktree remove</code> on {target?.path}. The branch is kept.
          </DialogDescription>
        </DialogHeader>
        <Label className="flex items-center gap-2 font-normal">
          <input type="checkbox" checked={force} onChange={(e) => setForce(e.target.checked)} />
          Force (discard uncommitted changes)
        </Label>
        <DialogFooter>
          <Button variant="outline" onClick={close}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            disabled={remove.isPending}
            onClick={() =>
              target &&
              remove.mutate({ projectId: target.projectId, path: target.path, force }, { onSuccess: close })
            }
          >
            {remove.isPending ? <Loader2 className="animate-spin" /> : <Trash2 />}
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
