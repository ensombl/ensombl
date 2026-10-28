import { useState } from "react"
import { Check, MessageSquare, RotateCcw, Trash2 } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { cn } from "@/lib/utils"
import type { AnchoredComment } from "@/shared/bindings/AnchoredComment"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { currentRange } from "../model/comment-range"
import { useAddComment, useDeleteComment, useResolveComment } from "../commands/peek-commands"

function timeAgo(ms: number) {
  const s = Math.round((Date.now() - ms) / 1000)
  if (s < 60) return "just now"
  if (s < 3600) return `${Math.round(s / 60)}m ago`
  if (s < 86400) return `${Math.round(s / 3600)}h ago`
  return `${Math.round(s / 86400)}d ago`
}

export function CommentCard({ item }: { item: AnchoredComment }) {
  const worktree = useWorkspace()
  const resolve = useResolveComment()
  const remove = useDeleteComment()
  const { comment } = item
  const range = currentRange(item)

  return (
    <div className={cn("rounded-md border bg-card p-3 text-sm shadow-sm", comment.resolved && "opacity-60")}>
      <div className="mb-1 flex items-center gap-2 text-xs text-muted-foreground">
        <MessageSquare className="size-3.5" />
        {range && range.end > range.start && (
          <span>
            Lines {range.start}–{range.end}
          </span>
        )}
        <span>{timeAgo(comment.createdAt)}</span>
        {item.currentLine === null && <span className="rounded bg-amber-500/15 px-1 text-amber-300">outdated</span>}
        {comment.resolved && <span className="rounded bg-emerald-500/15 px-1 text-emerald-300">resolved</span>}
        <div className="ml-auto flex gap-0.5">
          <Button
            variant="ghost"
            size="icon"
            className="size-6"
            title={comment.resolved ? "Reopen" : "Resolve"}
            onClick={() => resolve.mutate({ worktreeId: worktree.id, commentId: comment.id, resolved: !comment.resolved })}
          >
            {comment.resolved ? <RotateCcw className="size-3.5" /> : <Check className="size-3.5" />}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="size-6"
            title="Delete"
            onClick={() => remove.mutate({ worktreeId: worktree.id, commentId: comment.id })}
          >
            <Trash2 className="size-3.5" />
          </Button>
        </div>
      </div>
      <p className="whitespace-pre-wrap">{comment.body}</p>
    </div>
  )
}

export function CommentForm({ file, line, endLine = line, onDone }: { file: string; line: number; endLine?: number; onDone: () => void }) {
  const worktree = useWorkspace()
  const add = useAddComment()
  const [body, setBody] = useState("")
  const submit = () =>
    body.trim() && add.mutate({ worktreeId: worktree.id, file, line, endLine: endLine > line ? endLine : undefined, body }, { onSuccess: onDone })

  return (
    <div className="space-y-2 rounded-md border bg-card p-2 shadow-sm">
      <Textarea
        autoFocus
        rows={3}
        value={body}
        placeholder={endLine > line ? `Comment on lines ${line}–${endLine}…` : `Comment on line ${line}…`}
        onChange={(e) => setBody(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) submit()
          if (e.key === "Escape") onDone()
        }}
        className="resize-none text-sm"
      />
      <div className="flex justify-end gap-2">
        <Button size="sm" variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button size="sm" onClick={submit} disabled={!body.trim() || add.isPending}>
          Comment <kbd className="ml-1 font-mono text-[10px] opacity-70">⌘↵</kbd>
        </Button>
      </div>
    </div>
  )
}
