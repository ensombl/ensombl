import { useState } from "react"
import { ListTodo, Plus, Workflow } from "lucide-react"

import { Button } from "@/components/ui/button"
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable"
import { PaneEmpty } from "@/shared/ui/pane"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { useTaskEvents } from "../model/use-task-events"
import { useCoordination, useTasks } from "../queries/task-queries"
import { NewTaskDialog } from "./new-task-dialog"
import { TaskBoard } from "./task-board"
import { RunControls } from "./run-controls"
import { RunWorkflowDialog } from "./run-workflow-dialog"
import { TaskDetailPanel } from "./task-detail"

/** The project's orchestrated tasks: a board, and the selected task beside it. */
export function TasksView() {
  const worktree = useWorkspace()
  const { data: tasks = [] } = useTasks(worktree.projectId)
  const [selected, setSelected] = useState<string | null>(null)
  const [creating, setCreating] = useState(false)
  const [runningWorkflow, setRunningWorkflow] = useState(false)
  const running = tasks.filter((t) => t.status === "running" || t.status === "needsInput" || t.status === "queued").length
  const live = tasks.some((t) => t.status === "running" || t.status === "needsInput" || t.status === "review")
  const { data: coordination = [] } = useCoordination(worktree.projectId, live)
  useTaskEvents()
  const open = tasks.some((t) => t.id === selected) ? selected : null

  const dialog = (
    <NewTaskDialog
      projectId={worktree.projectId}
      defaultBase={worktree.branch ?? "main"}
      tasks={tasks}
      open={creating}
      onOpenChange={setCreating}
      onCreated={(task) => setSelected(task.id)}
    />
  )
  const workflowDialog = (
    <RunWorkflowDialog
      projectId={worktree.projectId}
      defaultBase={worktree.branch ?? "main"}
      open={runningWorkflow}
      onOpenChange={setRunningWorkflow}
    />
  )

  if (tasks.length === 0) {
    return (
      <>
        <PaneEmpty
          icon={<ListTodo />}
          title="No tasks yet"
          description="Hand work to agents. Each task runs on its own branch while you keep working, and merges only when you approve it."
        >
          <div className="flex gap-2">
            <Button size="sm" onClick={() => setCreating(true)}>
              <Plus /> New task
            </Button>
            <Button size="sm" variant="outline" onClick={() => setRunningWorkflow(true)}>
              <Workflow /> Run workflow
            </Button>
          </div>
        </PaneEmpty>
        {dialog}
        {workflowDialog}
      </>
    )
  }

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full">
      <ResizablePanel id="task-board" minSize="30" className="flex min-w-0 flex-col">
        <div className="flex h-10 shrink-0 items-center gap-2 border-b px-3">
          <span className="text-xs text-muted-foreground">
            {running} running · {tasks.filter((t) => t.status === "review").length} to review
          </span>
          <RunControls projectId={worktree.projectId} running={running} />
          <Button size="sm" variant="outline" className="ml-auto h-7" onClick={() => setRunningWorkflow(true)}>
            <Workflow /> Run workflow
          </Button>
          <Button size="sm" className="h-7" onClick={() => setCreating(true)}>
            <Plus /> New task
          </Button>
        </div>
        <div className="min-h-0 flex-1">
          <TaskBoard tasks={tasks} coordination={coordination} selected={open} onSelect={setSelected} />
        </div>
      </ResizablePanel>
      {open && (
        <>
          <ResizableHandle />
          <ResizablePanel id="task-detail" defaultSize="40" minSize="25" className="min-w-0">
            <TaskDetailPanel
              key={open}
              taskId={open}
              coordination={coordination.find((c) => c.taskId === open)}
              titleOf={(id) => tasks.find((t) => t.id === id)?.title ?? id}
              onClose={() => setSelected(null)}
            />
          </ResizablePanel>
        </>
      )}
      {dialog}
      {workflowDialog}
    </ResizablePanelGroup>
  )
}
