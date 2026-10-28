import { useState } from "react"
import {
  AlertTriangle,
  Ban,
  Check,
  CircleDot,
  GitBranch,
  GitMerge,
  Loader2,
  Play,
  RotateCcw,
  Send,
  ShieldQuestion,
  SquareTerminal,
  Trash2,
  Wrench,
  X,
} from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Textarea } from "@/components/ui/textarea"
import { cn } from "@/lib/utils"
import type { PendingInput } from "@/shared/bindings/PendingInput"
import type { Task } from "@/shared/bindings/Task"
import type { TaskCoordination } from "@/shared/bindings/TaskCoordination"
import { PaneEmpty } from "@/shared/ui/pane"

import { useStartTerminal } from "@/features/agents/commands/terminal-commands"
import { useTerminals } from "@/features/agents/queries/agent-queries"
import { TerminalView } from "@/features/agents/ui/terminal-view"

import { useTaskCommands } from "../commands/task-commands"
import { toActivity, type ActivityItem } from "../model/activity"
import { STATUS_LABEL } from "../model/board"
import { useRunLog, useTask, useTaskDiff } from "../queries/task-queries"

function ActivityRow({ item }: { item: ActivityItem }) {
  switch (item.kind) {
    case "text":
      return <p className="text-xs leading-relaxed whitespace-pre-wrap">{item.text}</p>
    case "tool":
      return (
        <p className="flex items-center gap-1.5 font-mono text-[11px] text-muted-foreground">
          <Wrench className="size-3 shrink-0" />
          <span className="text-foreground">{item.name}</span>
          <span className="truncate">{item.target}</span>
        </p>
      )
    case "error":
      return <p className="font-mono text-[11px] whitespace-pre-wrap text-red-300">{item.text}</p>
    case "notice":
      return <p className="text-[11px] text-muted-foreground">{item.text}</p>
    case "permission":
      return (
        <p className="flex items-center gap-1.5 text-[11px] text-amber-200">
          <ShieldQuestion className="size-3" /> Asked to use {item.tool}
          {item.allowed !== undefined && <span className="text-muted-foreground">· {item.allowed ? "allowed" : "denied"}</span>}
        </p>
      )
    case "result":
      return (
        <div className={cn("rounded-md border p-2 text-xs", item.ok ? "border-emerald-500/30" : "border-red-500/40")}>
          <p className="mb-1 flex items-center gap-1.5 text-[11px] text-muted-foreground">
            {item.ok ? <Check className="size-3 text-emerald-400" /> : <X className="size-3 text-red-400" />}
            {item.ok ? "Finished" : "Failed"}
            {item.durationMs !== null && ` · ${Math.round(item.durationMs / 1000)}s`}
            {item.costUsd !== null && ` · $${item.costUsd.toFixed(2)}`}
          </p>
          {item.text && <p className="whitespace-pre-wrap">{item.text}</p>}
        </div>
      )
    case "exit":
      return null
  }
}

function Activity({ runId }: { runId: string | null }) {
  const log = useRunLog(runId)
  const items = toActivity(log.data ?? [])
  if (!runId) return <p className="p-3 text-xs text-muted-foreground">Not started yet.</p>
  return (
    <div className="space-y-2 p-3">
      {items.map((item, i) => (
        <ActivityRow key={i} item={item} />
      ))}
      {items.length === 0 && <p className="text-xs text-muted-foreground">{log.isLoading ? "Loading…" : "Starting…"}</p>}
    </div>
  )
}

function Diff({ task }: { task: Task }) {
  const diff = useTaskDiff(task.id, task.status !== "backlog" && task.status !== "queued")
  if (diff.isLoading) return <Loader2 className="m-3 size-4 animate-spin text-muted-foreground" />
  if (!diff.data?.trim()) return <p className="p-3 text-xs text-muted-foreground">No committed changes yet. Work is committed when the agent finishes.</p>
  return (
    <pre className="min-w-max p-3 font-mono text-[11.5px] leading-5">
      {diff.data.split("\n").map((line, i) => (
        <div
          key={i}
          className={cn(
            line.startsWith("+") && !line.startsWith("+++") && "bg-emerald-500/10 text-emerald-300",
            line.startsWith("-") && !line.startsWith("---") && "bg-red-500/10 text-red-300",
            line.startsWith("@@") && "text-sky-300",
            (line.startsWith("diff ") || line.startsWith("+++") || line.startsWith("---")) && "font-semibold text-foreground",
          )}
        >
          {line || " "}
        </div>
      ))}
    </pre>
  )
}

/** Take over: an interactive agent in the task's worktree, continuing its session. */
function TakeOver({ task }: { task: Task }) {
  const worktreeId = `task:${task.id}`
  const { data: terminals = [] } = useTerminals(worktreeId)
  const start = useStartTerminal()
  const terminal = terminals[terminals.length - 1]
  const busy = task.status === "running" || task.status === "needsInput"
  const canStart = !!task.worktree && !busy && task.status !== "done"
  if (terminal) {
    return <TerminalView key={terminal.id} terminalId={terminal.id} visible onRestart={() => undefined} onClose={() => undefined} />
  }
  return (
    <PaneEmpty
      icon={<SquareTerminal />}
      title="Take over in a terminal"
      description={
        busy
          ? "Available once the agent stops."
          : canStart
            ? "Continue the agent's session interactively in the task's worktree. Your edits are committed with the task."
            : "The task has no worktree."
      }
    >
      <Button
        size="sm"
        disabled={!canStart || start.isPending}
        onClick={() => start.mutate({ worktreeId, profileId: "claude", cols: 100, rows: 30, resume: task.sessionId ?? undefined })}
      >
        <SquareTerminal /> Open terminal
      </Button>
    </PaneEmpty>
  )
}

function Pending({ task, pending }: { task: Task; pending: PendingInput }) {
  const { respond } = useTaskCommands()
  const answer = (allow: boolean) =>
    respond.mutate({ taskId: task.id, requestId: pending.requestId, decision: { allow, remember: false } })
  return (
    <div className="space-y-2 border-b bg-amber-400/5 p-3">
      <p className="flex items-center gap-1.5 text-xs font-medium text-amber-200">
        <ShieldQuestion className="size-3.5" /> The agent wants to use {pending.toolName}
      </p>
      {pending.description && <p className="text-xs text-muted-foreground">{pending.description}</p>}
      <pre className="max-h-40 overflow-auto rounded-md bg-muted/40 p-2 font-mono text-[11px] whitespace-pre-wrap">
        {JSON.stringify(pending.input, null, 2)}
      </pre>
      <div className="flex gap-2">
        <Button size="sm" className="h-7" disabled={respond.isPending} onClick={() => answer(true)}>
          <Check /> Allow
        </Button>
        <Button size="sm" variant="outline" className="h-7" disabled={respond.isPending} onClick={() => answer(false)}>
          <X /> Deny
        </Button>
      </div>
    </div>
  )
}

function FollowUp({ task }: { task: Task }) {
  const { followUp } = useTaskCommands()
  const [text, setText] = useState("")
  const send = () => followUp.mutate({ taskId: task.id, prompt: text }, { onSuccess: () => setText("") })
  return (
    <div className="flex items-end gap-2 border-t p-2">
      <Textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && (e.metaKey || e.ctrlKey) && text.trim() && send()}
        placeholder="Ask for changes before merging… (⌘↵)"
        className="min-h-9 resize-none text-xs"
        rows={2}
      />
      <Button size="icon" className="size-8 shrink-0" disabled={!text.trim() || followUp.isPending} onClick={send} aria-label="Send follow-up">
        <Send />
      </Button>
    </div>
  )
}

/** Where the task stands against its base and the other tasks. */
function Coordination({ task, coordination, titleOf }: { task: Task; coordination: TaskCoordination; titleOf: (id: string) => string }) {
  const { update } = useTaskCommands()
  const { behind, conflicts, overlaps } = coordination
  if (!behind && !conflicts.length && !overlaps.length) return null
  const idle = task.status === "review"
  return (
    <div className="space-y-1.5 border-b p-3 text-xs">
      {behind > 0 && (
        <p className="flex items-center gap-2 text-muted-foreground">
          <GitBranch className="size-3.5" />
          {behind} commit{behind > 1 ? "s" : ""} behind {task.baseBranch}
          {idle && !conflicts.length && (
            <Button size="sm" variant="outline" className="ml-auto h-6 text-xs" disabled={update.isPending} onClick={() => update.mutate({ taskId: task.id })}>
              <RotateCcw /> Update from {task.baseBranch}
            </Button>
          )}
        </p>
      )}
      {conflicts.length > 0 && (
        <p className="flex gap-2 text-red-300">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>
            Conflicts with {task.baseBranch} in <span className="font-mono">{conflicts.join(", ")}</span>. Send a follow-up asking the agent to merge {task.baseBranch} and resolve them.
          </span>
        </p>
      )}
      {overlaps.map((o) => (
        <p key={o.taskId} className="flex gap-2 text-amber-200">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>
            Also changed by “{titleOf(o.taskId)}”: <span className="font-mono">{o.files.slice(0, 5).join(", ")}</span>
            {o.files.length > 5 && ` +${o.files.length - 5}`}
          </span>
        </p>
      ))}
    </div>
  )
}

/** One task: what it's doing, what it changed, and what you can do about it. */
export function TaskDetailPanel({
  taskId,
  coordination,
  titleOf,
  onClose,
}: {
  taskId: string
  coordination?: TaskCoordination
  titleOf: (id: string) => string
  onClose: () => void
}) {
  const { data } = useTask(taskId)
  const commands = useTaskCommands()
  const [runId, setRunId] = useState<string | null>(null)
  if (!data) return <Loader2 className="m-4 size-4 animate-spin text-muted-foreground" />
  const { task, runs, pending } = data
  const run = runs.find((r) => r.id === runId) ?? runs[runs.length - 1]
  const status = task.status
  const id = task.id

  return (
    <div className="flex h-full flex-col">
      <header className="space-y-2 border-b p-3">
        <div className="flex items-start gap-2">
          <h2 className="min-w-0 flex-1 text-sm leading-snug font-semibold">{task.title}</h2>
          <Button variant="ghost" size="icon" className="-mt-1 size-7 shrink-0" onClick={onClose} aria-label="Close task">
            <X />
          </Button>
        </div>
        <div className="flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
          <Badge variant={status === "failed" ? "destructive" : "secondary"} className="gap-1">
            {status === "running" ? <Loader2 className="size-3 animate-spin" /> : <CircleDot className="size-3" />}
            {STATUS_LABEL[status]}
          </Badge>
          <span className="flex items-center gap-1 font-mono">
            <GitBranch className="size-3" />
            {task.branch} → {task.baseBranch}
          </span>
        </div>
        {task.error && (
          <p className="flex gap-1.5 text-xs text-red-300">
            <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
            {task.error}
          </p>
        )}
        <div className="flex flex-wrap gap-1.5">
          {status === "backlog" && (
            <Button size="sm" className="h-7" onClick={() => commands.queue.mutate({ taskId: id })}>
              <Play /> Queue
            </Button>
          )}
          {status === "review" && (
            <Button size="sm" className="h-7" disabled={commands.approve.isPending} onClick={() => commands.approve.mutate({ taskId: id })}>
              {commands.approve.isPending ? <Loader2 className="animate-spin" /> : <GitMerge />} Approve and merge
            </Button>
          )}
          {(status === "failed" || status === "cancelled") && (
            <Button size="sm" className="h-7" onClick={() => commands.retry.mutate({ taskId: id })}>
              <RotateCcw /> Retry
            </Button>
          )}
          {(status === "queued" || status === "running" || status === "needsInput") && (
            <Button size="sm" variant="outline" className="h-7" onClick={() => commands.cancel.mutate({ taskId: id })}>
              <Ban /> Cancel
            </Button>
          )}
          <Button
            size="sm"
            variant="ghost"
            className="h-7 text-muted-foreground"
            onClick={() => commands.remove.mutate({ taskId: id }, { onSuccess: onClose })}
            title={status === "done" ? "Remove from the board" : "Throw away the task, its worktree and its branch"}
          >
            <Trash2 /> {status === "done" ? "Remove" : "Discard"}
          </Button>
        </div>
      </header>
      {pending && <Pending task={task} pending={pending} />}
      {coordination && <Coordination task={task} coordination={coordination} titleOf={titleOf} />}
      <Tabs defaultValue="activity" className="min-h-0 flex-1 gap-0">
        <div className="flex items-center gap-2 border-b px-2 py-1.5">
          <TabsList className="h-7">
            <TabsTrigger value="activity" className="px-2 text-xs">
              Activity
            </TabsTrigger>
            <TabsTrigger value="changes" className="px-2 text-xs">
              Changes
            </TabsTrigger>
            <TabsTrigger value="terminal" className="px-2 text-xs">
              Take over
            </TabsTrigger>
            <TabsTrigger value="prompt" className="px-2 text-xs">
              Prompt
            </TabsTrigger>
          </TabsList>
          {runs.length > 1 && (
            <Select value={run?.id} onValueChange={setRunId}>
              <SelectTrigger size="sm" className="ml-auto h-7 w-24 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {runs.map((r, i) => (
                  <SelectItem key={r.id} value={r.id} className="text-xs">
                    Run {i + 1}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </div>
        <TabsContent value="activity" className="min-h-0 overflow-y-auto">
          <Activity runId={run?.id ?? null} />
        </TabsContent>
        <TabsContent value="changes" className="min-h-0 overflow-auto">
          <Diff task={task} />
        </TabsContent>
        <TabsContent value="terminal" className="min-h-0">
          <TakeOver task={task} />
        </TabsContent>
        <TabsContent value="prompt" className="min-h-0 space-y-3 overflow-y-auto p-3">
          {runs.length === 0 && <p className="text-xs whitespace-pre-wrap">{task.prompt}</p>}
          {runs.map((r, i) => (
            <div key={r.id} className="space-y-1">
              <p className="text-[11px] font-medium text-muted-foreground uppercase">{i === 0 ? "Prompt" : `Follow-up ${i}`}</p>
              <p className="text-xs whitespace-pre-wrap">{r.prompt}</p>
            </div>
          ))}
        </TabsContent>
      </Tabs>
      {status === "review" && <FollowUp task={task} />}
    </div>
  )
}
