import { useState, type ReactNode } from "react"
import { Loader2, Plus } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { Textarea } from "@/components/ui/textarea"
import type { PermissionMode } from "@/shared/bindings/PermissionMode"
import type { Task } from "@/shared/bindings/Task"

import { useBranches } from "@/features/workspaces/queries/workspace-queries"

import { useTaskCommands } from "../commands/task-commands"

const PERMISSIONS: { value: PermissionMode; label: string; hint: string }[] = [
  { value: "acceptEdits", label: "Edit freely, ask for the rest", hint: "File edits go through; commands and other tools wait for you." },
  { value: "default", label: "Ask before anything", hint: "Every tool that isn't read-only waits for you." },
  { value: "bypassPermissions", label: "Don't ask", hint: "The agent runs anything in its worktree. Only for trusted prompts." },
  { value: "plan", label: "Plan only", hint: "The agent investigates and proposes a plan without changing files." },
]

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid gap-1.5">
      <Label className="text-xs">{label}</Label>
      {children}
    </div>
  )
}

interface NewTaskDialogProps {
  projectId: string
  /** Branch new work starts from, usually the open worktree's. */
  defaultBase: string
  tasks: Task[]
  open: boolean
  onOpenChange: (open: boolean) => void
  onCreated: (task: Task) => void
}

/** Writes a task for an agent to run on its own branch. */
export function NewTaskDialog({ projectId, defaultBase, tasks, open, onOpenChange, onCreated }: NewTaskDialogProps) {
  const { create } = useTaskCommands()
  const branches = useBranches(projectId, open)
  const [title, setTitle] = useState("")
  const [prompt, setPrompt] = useState("")
  const [base, setBase] = useState(defaultBase)
  const [permission, setPermission] = useState<PermissionMode>("acceptEdits")
  const [dependsOn, setDependsOn] = useState<string[]>([])
  const [queue, setQueue] = useState(true)
  const waitable = tasks.filter((t) => t.status !== "done" && t.status !== "cancelled")

  const submit = () =>
    create.mutate(
      { projectId, title, prompt, agent: "claude", baseBranch: base, dependsOn, permission, queue },
      {
        onSuccess: (task) => {
          onCreated(task)
          onOpenChange(false)
          setTitle("")
          setPrompt("")
          setDependsOn([])
        },
      },
    )

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>New task</DialogTitle>
          <DialogDescription>An agent works on it alone in its own worktree and branch. You review the result before it merges.</DialogDescription>
        </DialogHeader>
        <form
          className="grid gap-4"
          onSubmit={(e) => {
            e.preventDefault()
            submit()
          }}
        >
          <Field label="Title">
            <Input autoFocus value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Add rate limiting to the claims API" />
          </Field>
          <Field label="Prompt">
            <Textarea
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="What should the agent do, and how will you know it's done?"
              className="min-h-32 text-sm"
              onKeyDown={(e) => e.key === "Enter" && (e.metaKey || e.ctrlKey) && submit()}
            />
          </Field>
          <div className="grid grid-cols-2 gap-3">
            <Field label="Start from">
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
            </Field>
            <Field label="Waits for">
              <Popover>
                <PopoverTrigger asChild>
                  <Button type="button" variant="outline" className="justify-start text-xs font-normal" disabled={waitable.length === 0}>
                    {dependsOn.length === 0 ? "Nothing" : `${dependsOn.length} task${dependsOn.length > 1 ? "s" : ""}`}
                  </Button>
                </PopoverTrigger>
                <PopoverContent align="start" className="max-h-64 w-72 overflow-y-auto p-1">
                  {waitable.map((t) => (
                    <label key={t.id} className="flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-xs hover:bg-accent">
                      <Checkbox
                        checked={dependsOn.includes(t.id)}
                        onCheckedChange={(checked) => setDependsOn((d) => (checked ? [...d, t.id] : d.filter((id) => id !== t.id)))}
                      />
                      <span className="truncate">{t.title}</span>
                    </label>
                  ))}
                </PopoverContent>
              </Popover>
            </Field>
          </div>
          <Field label="Permissions">
            <Select value={permission} onValueChange={(v) => setPermission(v as PermissionMode)}>
              <SelectTrigger className="w-full text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PERMISSIONS.map((p) => (
                  <SelectItem key={p.value} value={p.value} className="text-xs">
                    {p.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">{PERMISSIONS.find((p) => p.value === permission)?.hint}</p>
          </Field>
          <DialogFooter className="items-center sm:justify-between">
            <Label className="flex items-center gap-2 text-xs font-normal">
              <Switch checked={queue} onCheckedChange={setQueue} /> Start when a slot is free
            </Label>
            <Button type="submit" disabled={!title.trim() || !prompt.trim() || create.isPending}>
              {create.isPending ? <Loader2 className="animate-spin" /> : <Plus />} Create task
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
