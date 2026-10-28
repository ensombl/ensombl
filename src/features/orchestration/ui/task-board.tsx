import { GitBranch, Link2, Loader2 } from "lucide-react"

import { cn } from "@/lib/utils"
import type { Task } from "@/shared/bindings/Task"

import { AgentIcon } from "@/features/agents/ui/agent-icon"

import { byColumn, COLUMNS, STATUS_LABEL } from "../model/board"

function ago(ms: number) {
  const s = Math.round((Date.now() - ms) / 1000)
  if (s < 60) return "just now"
  if (s < 3600) return `${Math.round(s / 60)}m ago`
  if (s < 86400) return `${Math.round(s / 3600)}h ago`
  return `${Math.round(s / 86400)}d ago`
}

function TaskCard({ task, selected, onSelect }: { task: Task; selected: boolean; onSelect: () => void }) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "w-full space-y-1.5 rounded-lg border bg-card p-2.5 text-left text-card-foreground hover:border-ring/60",
        selected && "border-ring ring-1 ring-ring",
      )}
    >
      <div className="flex items-start gap-1.5">
        {task.status === "running" ? (
          <Loader2 className="mt-0.5 size-3.5 shrink-0 animate-spin text-muted-foreground" />
        ) : (
          <AgentIcon kind={task.agent} className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" />
        )}
        <span className="min-w-0 flex-1 text-xs leading-snug font-medium break-words">{task.title}</span>
        {task.status === "needsInput" && <span className="mt-1 size-2 shrink-0 rounded-full bg-amber-400" title={STATUS_LABEL.needsInput} />}
      </div>
      <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
        <span className="flex min-w-0 items-center gap-1 font-mono">
          <GitBranch className="size-3 shrink-0" />
          <span className="truncate">{task.branch.replace(/^ensombl\//, "")}</span>
        </span>
        {task.dependsOn.length > 0 && (
          <span className="flex shrink-0 items-center gap-0.5" title="Waits for other tasks">
            <Link2 className="size-3" />
            {task.dependsOn.length}
          </span>
        )}
        <span className="ml-auto shrink-0">{ago(task.updatedAt)}</span>
      </div>
      {task.status === "failed" && task.error && <p className="line-clamp-2 text-[11px] text-red-300">{task.error}</p>}
    </button>
  )
}

/** Tasks in columns by status. */
export function TaskBoard({ tasks, selected, onSelect }: { tasks: Task[]; selected: string | null; onSelect: (id: string) => void }) {
  const columns = byColumn(tasks)
  return (
    <div className="flex h-full gap-2 overflow-x-auto p-2">
      {COLUMNS.map((column) => (
        <section key={column.id} className="flex w-56 shrink-0 flex-col rounded-lg bg-muted/30">
          <h3 className="flex items-center gap-1.5 px-2.5 py-2 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
            {column.label}
            <span className="font-normal">{columns[column.id].length || ""}</span>
          </h3>
          <div className="min-h-0 flex-1 space-y-1.5 overflow-y-auto px-1.5 pb-1.5">
            {columns[column.id].map((task) => (
              <TaskCard key={task.id} task={task} selected={task.id === selected} onSelect={() => onSelect(task.id)} />
            ))}
          </div>
        </section>
      ))}
    </div>
  )
}
