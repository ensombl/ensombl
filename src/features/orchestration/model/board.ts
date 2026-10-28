import type { Task } from "@/shared/bindings/Task"
import type { TaskStatus } from "@/shared/bindings/TaskStatus"

export type ColumnId = "backlog" | "queued" | "running" | "review" | "done" | "stopped"

export const COLUMNS: { id: ColumnId; label: string }[] = [
  { id: "backlog", label: "Backlog" },
  { id: "queued", label: "Queued" },
  { id: "running", label: "Running" },
  { id: "review", label: "Review" },
  { id: "done", label: "Done" },
  { id: "stopped", label: "Failed or cancelled" },
]

const COLUMN_OF: Record<TaskStatus, ColumnId> = {
  backlog: "backlog",
  queued: "queued",
  running: "running",
  needsInput: "running",
  review: "review",
  done: "done",
  failed: "stopped",
  cancelled: "stopped",
}

/** Tasks by board column; newest activity first within each. */
export function byColumn(tasks: Task[]): Record<ColumnId, Task[]> {
  const columns = Object.fromEntries(COLUMNS.map((c) => [c.id, [] as Task[]])) as Record<ColumnId, Task[]>
  for (const task of tasks) columns[COLUMN_OF[task.status]].push(task)
  for (const list of Object.values(columns)) list.sort((a, b) => b.updatedAt - a.updatedAt)
  return columns
}

export const STATUS_LABEL: Record<TaskStatus, string> = {
  backlog: "Backlog",
  queued: "Queued",
  running: "Running",
  needsInput: "Needs input",
  review: "In review",
  done: "Done",
  failed: "Failed",
  cancelled: "Cancelled",
}
