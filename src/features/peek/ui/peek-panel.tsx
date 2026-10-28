import { useState } from "react"
import { Code2, ExternalLink, Loader2, MessageSquareShare, Settings2, X } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { Pane, PaneEmpty } from "@/shared/ui/pane"
import { usePromptTags } from "@/shared/workspace/prompt-tags"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { useOpenInEditor, useUpdateSettings } from "../commands/peek-commands"
import { usePeek, type PeekTarget } from "../model/peek-store"
import { reviewPrompt } from "../model/review-prompt"
import { useComments, useSettings, useSource } from "../queries/peek-queries"
import { CodeView } from "./code-view"

function EditorSettings() {
  const { data } = useSettings()
  const update = useUpdateSettings()
  const [value, setValue] = useState<string | null>(null)
  const command = value ?? data?.editorCommand ?? ""

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="icon" className="size-7" aria-label="Editor settings">
          <Settings2 />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80 space-y-2">
        <Label htmlFor="editor-command">Editor command</Label>
        <Input
          id="editor-command"
          value={command}
          onChange={(e) => setValue(e.target.value)}
          className="font-mono text-xs"
          spellCheck={false}
        />
        <p className="text-xs text-muted-foreground">
          <code>{"{file}"}</code> and <code>{"{line}"}</code> are filled in. Terminal editors (nvim, vim, hx) open in an
          agent tab; GUI editors (code, cursor, zed) open in their own window.
        </p>
        <Button size="sm" className="w-full" disabled={update.isPending} onClick={() => update.mutate({ editorCommand: command })}>
          Save
        </Button>
      </PopoverContent>
    </Popover>
  )
}

export function PeekPanel({ target }: { target: PeekTarget }) {
  const worktree = useWorkspace()
  const { close } = usePeek()
  const source = useSource(worktree.id, target.file)
  const comments = useComments(worktree.id)
  const openInEditor = useOpenInEditor()
  const { appendDraft } = usePromptTags()

  const fileComments = (comments.data ?? []).filter((c) => c.comment.file === target.file)
  const openCount = (comments.data ?? []).filter((c) => !c.comment.resolved).length

  const sendReview = () => {
    const text = reviewPrompt(comments.data ?? [], { [target.file]: source.data?.content })
    if (text) appendDraft(text)
  }

  return (
    <Pane
      title={target.title}
      icon={<Code2 />}
      actions={
        <>
          <span className="truncate font-mono text-xs text-muted-foreground">
            {target.file}:{target.line}
          </span>
          {openCount > 0 && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="outline" size="sm" className="h-7 gap-1.5" onClick={sendReview}>
                  <MessageSquareShare /> Send review
                  <Badge variant="secondary" className="h-4 px-1 text-[10px]">
                    {openCount}
                  </Badge>
                </Button>
              </TooltipTrigger>
              <TooltipContent>Put all open comments in the agent prompt</TooltipContent>
            </Tooltip>
          )}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="size-7"
                aria-label="Open in editor"
                disabled={openInEditor.isPending}
                onClick={() => openInEditor.mutate({ worktreeId: worktree.id, path: target.file, line: target.line })}
              >
                <ExternalLink />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Open in editor</TooltipContent>
          </Tooltip>
          <EditorSettings />
          <Button variant="ghost" size="icon" className="size-7" onClick={close} aria-label="Close peek">
            <X />
          </Button>
        </>
      }
    >
      {source.data ? (
        <CodeView path={target.file} content={source.data.content} line={target.line} comments={fileComments} />
      ) : (
        <PaneEmpty
          icon={source.isError ? <Code2 /> : <Loader2 className="animate-spin" />}
          title={source.isError ? "Couldn't read file" : "Loading…"}
          description={source.isError ? String(source.error) : target.file}
        />
      )}
    </Pane>
  )
}
