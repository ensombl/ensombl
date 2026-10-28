import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useReducer, useRef, useState } from "react"
import { Brain, ChevronRight, CircleAlert, Info, RotateCcw, Sparkles, X } from "lucide-react"
import { toast } from "sonner"

import { Button } from "@/components/ui/button"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import { cn } from "@/lib/utils"
import type { ArchModel } from "@/shared/bindings/ArchModel"
import type { ChatEvent } from "@/shared/bindings/ChatEvent"
import type { ChatInfo } from "@/shared/bindings/ChatInfo"
import type { PatternInstance } from "@/shared/bindings/PatternInstance"
import type { PermissionDecision } from "@/shared/bindings/PermissionDecision"
import type { PermissionMode } from "@/shared/bindings/PermissionMode"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { usePeekStore } from "@/features/peek/model/peek-store"

import { attachChat, interruptChat, respondPermission, sendChatMessage, setChatModel, setChatPermissionMode } from "../../commands/chat-commands"
import { emptyChat, pendingPermissions, reduceAll, type ChatItem } from "../../model/chat-state"
import { useChatStatus } from "../../model/chat-status"
import { Composer } from "../composer"
import { ChatActionsContext, type ChatActions } from "./chat-actions"
import { ChatControls } from "./chat-controls"
import { CookingStatus } from "./cooking-status"
import { Markdown } from "./markdown"
import { PermissionCard, ToolCard } from "./tool-card"

type Action = { type: "reset"; events: ChatEvent[] } | { type: "events"; events: ChatEvent[] }

const reducer = (state: ReturnType<typeof emptyChat>, action: Action) =>
  action.type === "reset" ? reduceAll(action.events) : reduceAll(action.events, state)

const FLUSH_MS = 80

const CONTEXT_MARKER = "\n\n<architecture-context"

function UserMessage({ text }: { text: string }) {
  const cut = text.indexOf(CONTEXT_MARKER)
  const prompt = cut < 0 ? text : text.slice(0, cut)
  const context = cut < 0 ? null : text.slice(cut).trim()
  return (
    <div className="ml-auto max-w-[85%] space-y-1.5 rounded-lg bg-muted px-3 py-2 text-sm">
      <p className="whitespace-pre-wrap">{prompt}</p>
      {context && (
        <Collapsible>
          <CollapsibleTrigger className="group flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground">
            <ChevronRight className="size-3 transition-transform group-data-[state=open]:rotate-90" />
            Architecture context ({(context.match(/^### /gm) ?? []).length})
          </CollapsibleTrigger>
          <CollapsibleContent>
            <pre className="mt-1 max-h-60 overflow-auto font-mono text-[11px] whitespace-pre-wrap text-muted-foreground">{context}</pre>
          </CollapsibleContent>
        </Collapsible>
      )}
    </div>
  )
}

function Thinking({ text }: { text: string }) {
  // Hidden reasoning arrives empty; there's nothing worth showing.
  if (!text.trim()) return null
  return (
    <Collapsible>
      <CollapsibleTrigger className="group flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground">
        <ChevronRight className="size-3 transition-transform group-data-[state=open]:rotate-90" />
        <Brain className="size-3.5" /> Thinking
      </CollapsibleTrigger>
      <CollapsibleContent>
        <p className="mt-1.5 border-l pl-3 text-xs whitespace-pre-wrap text-muted-foreground italic">{text}</p>
      </CollapsibleContent>
    </Collapsible>
  )
}

const seconds = (ms: number) => (ms < 60_000 ? `${(ms / 1000).toFixed(1)}s` : `${Math.floor(ms / 60_000)}m ${Math.round((ms % 60_000) / 1000)}s`)

const Row = memo(function Row({ item }: { item: ChatItem }) {
  switch (item.kind) {
    case "user":
      return <UserMessage text={item.text} />
    case "text":
      return <Markdown text={item.text} />
    case "thinking":
      return <Thinking text={item.text} />
    case "tool":
      return <ToolCard item={item} />
    case "permission":
      return <PermissionCard request={item.request} />
    case "turn":
      if (item.isError) {
        return (
          <p className="flex items-start gap-1.5 text-xs text-destructive">
            <CircleAlert className="mt-0.5 size-3.5 shrink-0" /> {item.result || "The turn ended with an error"}
          </p>
        )
      }
      return item.durationMs ? <p className="text-[11px] text-muted-foreground/70">Done in {seconds(item.durationMs)}</p> : null
    case "notice":
      return (
        <p className={cn("flex items-start gap-1.5 text-xs whitespace-pre-wrap", item.level === "error" ? "text-destructive" : "text-muted-foreground")}>
          {item.level === "error" ? <CircleAlert className="mt-0.5 size-3.5 shrink-0" /> : <Info className="mt-0.5 size-3.5 shrink-0" />}
          {item.text}
        </p>
      )
  }
})

interface ChatViewProps {
  chat: ChatInfo
  visible: boolean
  model: ArchModel | null
  patterns: PatternInstance[]
  onRestart: (sessionId: string | null) => void
  onClose: () => void
}

/** A native chat with an agent: streamed replies, tool cards and inline approvals. */
export function ChatView({ chat, visible, model, patterns, onRestart, onClose }: ChatViewProps) {
  const worktree = useWorkspace()
  const openPeek = usePeekStore((s) => s.open)
  const [state, dispatch] = useReducer(reducer, undefined, emptyChat)
  const scroller = useRef<HTMLDivElement>(null)
  const atBottom = useRef(true)
  const [attached, setAttached] = useState(false)

  // Events arrive in bursts while text streams; apply them in batches. Every
  // frame would re-parse the growing markdown ~60 times a second.
  useEffect(() => {
    let disposed = false
    let queue: ChatEvent[] = []
    let frame: ReturnType<typeof setTimeout> | 0 = 0
    const flush = () => {
      frame = 0
      if (disposed || queue.length === 0) return
      const events = queue
      queue = []
      dispatch({ type: "events", events })
    }
    let ready = false
    attachChat(chat.id, (event) => {
      queue.push(event)
      if (ready && !frame) frame = setTimeout(flush, FLUSH_MS)
    })
      .then((snapshot) => {
        if (disposed) return
        dispatch({ type: "reset", events: snapshot.events })
        ready = true
        setAttached(true)
        flush()
      })
      .catch((error) => toast.error("Couldn't open chat", { description: String(error) }))
    return () => {
      disposed = true
      clearTimeout(frame)
    }
  }, [chat.id])

  const publish = useChatStatus((s) => s.set)
  const needsInput = useMemo(() => pendingPermissions(state.items).length > 0, [state.items])
  useEffect(() => {
    publish(chat.id, { busy: state.busy, running: state.running, needsInput })
  }, [publish, chat.id, state.busy, state.running, needsInput])

  useLayoutEffect(() => {
    const el = scroller.current
    if (el && atBottom.current) el.scrollTop = el.scrollHeight
  }, [state.items, visible])

  const actions = useMemo<ChatActions>(
    () => ({
      cwd: worktree.path,
      respond: (requestId: string, decision: PermissionDecision) =>
        void respondPermission(chat.id, requestId, decision).catch((e) => toast.error(String(e))),
      setMode: (mode: PermissionMode) => void setChatPermissionMode(chat.id, mode).catch((e) => toast.error(String(e))),
      openFile: (file: string, line = 1) => openPeek(worktree.id, { file, line, title: file.split("/").pop() ?? file }),
    }),
    [chat.id, worktree.id, worktree.path, openPeek],
  )

  const stop = useCallback(() => void interruptChat(chat.id), [chat.id])

  return (
    <ChatActionsContext.Provider value={actions}>
      <div className={cn("absolute inset-0 flex flex-col", !visible && "invisible")}>
        <div
          ref={scroller}
          onScroll={(e) => {
            const el = e.currentTarget
            atBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40
          }}
          className="min-h-0 flex-1 overflow-y-auto"
        >
          <div className="mx-auto flex max-w-3xl flex-col gap-3 p-4">
            {attached && state.items.length === 0 && (
              <div className="flex flex-col items-center gap-2 py-16 text-center text-sm text-muted-foreground">
                <Sparkles className="size-5" />
                <p>Ask {chat.title} anything about this worktree.</p>
                <p className="text-xs">Use @ to tag classes, methods, files and patterns from the map.</p>
              </div>
            )}
            {state.items.map((item) => (
              <Row key={item.id} item={item} />
            ))}
            {state.busy && state.turn && <CookingStatus key={state.turn.startedAt} turn={state.turn} waiting={needsInput} />}
            {!state.running && (
              <div className="flex items-center gap-2 rounded-lg border bg-card p-2 pl-3 text-sm">
                <span className="text-muted-foreground">The agent has exited.</span>
                <Button size="sm" variant="outline" className="ml-auto h-7" onClick={() => onRestart(state.sessionId)}>
                  <RotateCcw /> {state.sessionId ? "Resume" : "Restart"}
                </Button>
                <Button size="sm" variant="ghost" className="h-7" onClick={onClose}>
                  <X /> Close
                </Button>
              </div>
            )}
          </div>
        </div>
        <Composer
          model={model}
          patterns={patterns}
          disabled={!state.running}
          busy={state.busy}
          onStop={stop}
          controls={
            <ChatControls
              mode={state.mode}
              model={state.model}
              disabled={!state.running}
              onMode={actions.setMode}
              onModel={(m) => void setChatModel(chat.id, m).catch((e) => toast.error(String(e)))}
            />
          }
          placeholder={`Message ${chat.title}… @ to tag a class, method or file`}
          onSend={(prompt) => {
            atBottom.current = true
            return sendChatMessage(chat.id, prompt).catch((e) => {
              toast.error("Couldn't send", { description: String(e) })
              throw e
            })
          }}
        />
      </div>
    </ChatActionsContext.Provider>
  )
}
