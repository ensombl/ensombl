import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react"
import { ArrowUp, AtSign, FileCode2, Shapes, Square, X } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { KindBadge } from "@/features/architecture/ui/kind-badge"
import { cn } from "@/lib/utils"
import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { PatternInstance } from "@/shared/bindings/PatternInstance"
import { tagKey, usePromptTags } from "@/shared/workspace/prompt-tags"

import { buildPrompt } from "../model/context"
import { matchSuggestions, mentionAt, taggables, type Suggestion } from "../model/suggestions"

interface ComposerProps {
  model: ArchModel | null
  patterns: PatternInstance[]
  /** Receives the prompt with tag context expanded. */
  onSend: (prompt: string) => Promise<unknown> | void
  placeholder: string
  disabled?: boolean
  /** A turn is running; shows a stop button. */
  busy?: boolean
  onStop?: () => void
}

function SuggestionIcon({ s }: { s: Suggestion }) {
  if (s.kind === "file") return <FileCode2 className="size-4 text-muted-foreground" />
  if (s.kind === "member") return <AtSign className="size-4 text-muted-foreground" />
  if (s.kind === "pattern") return <Shapes className="size-4 text-violet-300" />
  return <KindBadge kind={s.kind} className="size-4 text-[9px]" />
}

/** Prompt box for the active agent with `@` tagging of classes, members and files. */
export function Composer({ model, patterns, onSend, placeholder, disabled, busy, onStop }: ComposerProps) {
  const { tags, add, remove, clear, draft, takeDraft } = usePromptTags()
  const [text, setText] = useState("")

  // Pick up text queued elsewhere (e.g. "Send review to agent").
  useEffect(() => {
    if (!draft) return
    const frame = requestAnimationFrame(() => {
      const queued = takeDraft()
      if (!queued) return
      setText((current) => (current.trim() ? `${current}\n\n${queued}` : queued))
      input.current?.focus()
    })
    return () => cancelAnimationFrame(frame)
  }, [draft]) // eslint-disable-line react-hooks/exhaustive-deps
  const [caret, setCaret] = useState(0)
  const [highlight, setHighlight] = useState(0)
  const input = useRef<HTMLTextAreaElement>(null)

  const all = useMemo(() => taggables(model, patterns), [model, patterns])
  const mention = mentionAt(text, caret)
  const suggestions = useMemo(() => (mention ? matchSuggestions(all, mention.query) : []), [all, mention?.query]) // eslint-disable-line react-hooks/exhaustive-deps
  const open = mention !== null && suggestions.length > 0
  const canSend = !disabled && (text.trim().length > 0 || tags.length > 0)

  const choose = (s: Suggestion) => {
    if (!mention) return
    const insert = `@${s.tag.label} `
    const next = text.slice(0, mention.start) + insert + text.slice(caret)
    const pos = mention.start + insert.length
    add(s.tag)
    setText(next)
    setCaret(pos)
    setHighlight(0)
    requestAnimationFrame(() => {
      input.current?.focus()
      input.current?.setSelectionRange(pos, pos)
    })
  }

  const send = async () => {
    if (!canSend) return
    try {
      await onSend(buildPrompt(text, tags, model, patterns))
    } catch {
      return // keep the draft so nothing is lost
    }
    setText("")
    setCaret(0)
    clear()
  }

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (open) {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault()
        const delta = e.key === "ArrowDown" ? 1 : -1
        setHighlight((h) => (h + delta + suggestions.length) % suggestions.length)
        return
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault()
        choose(suggestions[Math.min(highlight, suggestions.length - 1)])
        return
      }
      if (e.key === "Escape") {
        e.preventDefault()
        setCaret(-1)
        return
      }
    }
    if (e.key === "Escape" && busy && onStop) {
      e.preventDefault()
      onStop()
      return
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      void send()
    }
  }

  return (
    <div className="shrink-0 space-y-2 border-t bg-background p-2">
      {tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {tags.map((t) => (
            <Badge key={tagKey(t)} variant="secondary" className="gap-1 pr-1 font-mono text-xs">
              @{t.label}
              <button type="button" aria-label={`Remove ${t.label}`} onClick={() => remove(tagKey(t))} className="rounded-sm hover:bg-muted-foreground/20">
                <X className="size-3" />
              </button>
            </Badge>
          ))}
        </div>
      )}
      <div className="relative">
        {open && (
          <div className="absolute inset-x-0 bottom-full z-20 mb-1 overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md">
            {suggestions.map((s, i) => (
              <button
                key={tagKey(s.tag)}
                type="button"
                onMouseDown={(e) => {
                  e.preventDefault()
                  choose(s)
                }}
                onMouseEnter={() => setHighlight(i)}
                className={cn(
                  "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm",
                  i === highlight && "bg-accent text-accent-foreground",
                )}
              >
                <SuggestionIcon s={s} />
                <span className="font-medium">{s.tag.label}</span>
                <span className="ml-auto truncate pl-3 font-mono text-xs text-muted-foreground">{s.detail}</span>
              </button>
            ))}
          </div>
        )}
        <Textarea
          ref={input}
          value={text}
          rows={2}
          placeholder={placeholder}
          disabled={disabled}
          onChange={(e) => {
            setText(e.target.value)
            setCaret(e.target.selectionStart)
            setHighlight(0)
          }}
          onSelect={(e) => setCaret(e.currentTarget.selectionStart)}
          onKeyDown={onKeyDown}
          className={cn("max-h-40 min-h-16 resize-none text-sm", busy && onStop ? "pr-20" : "pr-12")}
        />
        <div className="absolute right-2 bottom-2 flex gap-1">
          {busy && onStop && (
            <Button size="icon" variant="secondary" className="size-7" onClick={onStop} aria-label="Stop (Esc)" title="Stop (Esc)">
              <Square className="fill-current" />
            </Button>
          )}
          <Button size="icon" className="size-7 rounded-full" disabled={!canSend} onClick={() => void send()} aria-label="Send to agent">
            <ArrowUp />
          </Button>
        </div>
      </div>
    </div>
  )
}
