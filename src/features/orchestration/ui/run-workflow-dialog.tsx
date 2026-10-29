import { useState } from "react"
import { Loader2, Workflow } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Textarea } from "@/components/ui/textarea"

import { useBranches } from "@/features/workspaces/queries/workspace-queries"

import { useTaskCommands } from "../commands/task-commands"
import { useWorkflows } from "../queries/task-queries"

interface RunWorkflowDialogProps {
  projectId: string
  defaultBase: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Starts one of the repo's workflows: its steps become dependent tasks. */
export function RunWorkflowDialog({ projectId, defaultBase, open, onOpenChange }: RunWorkflowDialogProps) {
  const { data: workflows = [] } = useWorkflows(projectId, open)
  const branches = useBranches(projectId, open)
  const { startWorkflow } = useTaskCommands()
  const [file, setFile] = useState<string | null>(null)
  const [input, setInput] = useState("")
  const [base, setBase] = useState(defaultBase)
  const usable = workflows.filter((w) => !w.error)
  const chosen = usable.find((w) => w.file === file) ?? usable[0]

  const start = () =>
    chosen &&
    startWorkflow.mutate(
      { projectId, workflow: chosen.file, input, baseBranch: base },
      {
        onSuccess: () => {
          onOpenChange(false)
          setInput("")
        },
      },
    )

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Run a workflow</DialogTitle>
          <DialogDescription>
            Workflows live in your repo under <code className="font-mono">.ensombl/workflows/*.yml</code>. Each step becomes a task that waits for the steps before it.
          </DialogDescription>
        </DialogHeader>
        {workflows.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            This project has no workflows yet. Add a YAML file to <code className="font-mono">.ensombl/workflows/</code>; see the README for the format.
          </p>
        ) : (
          <div className="grid gap-4">
            <div className="grid gap-1.5">
              <Label className="text-xs">Workflow</Label>
              <Select value={chosen?.file} onValueChange={setFile}>
                <SelectTrigger className="w-full text-xs">
                  <SelectValue placeholder="Pick a workflow" />
                </SelectTrigger>
                <SelectContent>
                  {usable.map((w) => (
                    <SelectItem key={w.file} value={w.file} className="text-xs">
                      {w.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {chosen && (
                <p className="text-xs text-muted-foreground">
                  {chosen.description ? `${chosen.description} · ` : ""}
                  {chosen.steps.join(" → ")}
                </p>
              )}
              {workflows
                .filter((w) => w.error)
                .map((w) => (
                  <p key={w.file} className="text-xs text-red-300">
                    {w.file}: {w.error}
                  </p>
                ))}
            </div>
            <div className="grid gap-1.5">
              <Label className="text-xs">What's it for?</Label>
              <Textarea value={input} onChange={(e) => setInput(e.target.value)} placeholder="Fills {{input}} in the workflow's prompts" className="min-h-24 text-sm" />
            </div>
            <div className="grid gap-1.5">
              <Label className="text-xs">Start from</Label>
              <Select value={base} onValueChange={setBase}>
                <SelectTrigger className="w-full font-mono text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {(branches.data ?? [base]).map((b) => (
                    <SelectItem key={b} value={b} className="font-mono text-xs">
                      {b}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        )}
        <DialogFooter>
          <Button disabled={!chosen || !input.trim() || startWorkflow.isPending} onClick={start}>
            {startWorkflow.isPending ? <Loader2 className="animate-spin" /> : <Workflow />} Run
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
