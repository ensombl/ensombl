import { memo, useState, type ReactNode } from "react"
import {
  Check,
  ChevronRight,
  Circle,
  CircleCheck,
  CircleDot,
  FilePen,
  FileText,
  Globe,
  ListTodo,
  Loader2,
  MessageCircleQuestion,
  Search,
  ShieldAlert,
  Sparkles,
  SquareTerminal,
  Wrench,
  X,
} from "lucide-react"

import { Button } from "@/components/ui/button"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import { cn } from "@/lib/utils"

import type { ChatItem, PermissionRequest } from "../../model/chat-state"
import { diffLines, fileEdits, questionsOf, summarizeTool, todosOf, type Question } from "../../model/tools"
import { useChatActions } from "./chat-actions"
import { CodeBlock, Markdown } from "./markdown"

type ToolItem = Extract<ChatItem, { kind: "tool" }>

const ICONS: Record<string, typeof Wrench> = {
  Bash: SquareTerminal,
  Read: FileText,
  Write: FilePen,
  Edit: FilePen,
  MultiEdit: FilePen,
  NotebookEdit: FilePen,
  Glob: Search,
  Grep: Search,
  WebFetch: Globe,
  WebSearch: Globe,
  Task: Sparkles,
  Agent: Sparkles,
  TodoWrite: ListTodo,
  AskUserQuestion: MessageCircleQuestion,
}

function Output({ text, isError }: { text: string; isError?: boolean }) {
  if (!text.trim()) return null
  return (
    <pre
      className={cn(
        "max-h-72 overflow-auto rounded-md border bg-muted/40 p-2.5 font-mono text-xs leading-relaxed whitespace-pre-wrap",
        isError && "border-destructive/40 text-destructive",
      )}
    >
      {text}
    </pre>
  )
}

function Diff({ before, after }: { before: string; after: string }) {
  const lines = diffLines(before, after)
  return (
    <pre className="max-h-96 overflow-auto rounded-md border bg-muted/40 py-1.5 font-mono text-xs leading-relaxed">
      {lines.map((l, i) => (
        <div
          key={i}
          className={cn(
            "px-2.5 whitespace-pre-wrap",
            l.op === "+" && "bg-emerald-500/10 text-emerald-300",
            l.op === "-" && "bg-red-500/10 text-red-300",
          )}
        >
          <span className="mr-2 inline-block w-2 text-muted-foreground select-none">{l.op}</span>
          {l.text || " "}
        </div>
      ))}
    </pre>
  )
}

function Todos({ input }: { input: unknown }) {
  const todos = todosOf(input)
  return (
    <ul className="space-y-1 text-sm">
      {todos.map((t, i) => (
        <li key={i} className="flex items-start gap-2">
          {t.status === "completed" ? (
            <CircleCheck className="mt-0.5 size-3.5 shrink-0 text-emerald-400" />
          ) : t.status === "in_progress" ? (
            <CircleDot className="mt-0.5 size-3.5 shrink-0 text-sky-400" />
          ) : (
            <Circle className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" />
          )}
          <span className={cn(t.status === "completed" && "text-muted-foreground line-through")}>{t.content}</span>
        </li>
      ))}
    </ul>
  )
}

function Details({ item }: { item: ToolItem }) {
  const input = (item.input ?? {}) as Record<string, unknown>
  const result = item.result
  const edits = fileEdits(item.name, item.input)
  if (item.name === "Bash") {
    return (
      <div className="space-y-2">
        <CodeBlock code={String(input.command ?? "")} language="bash" />
        {result && <Output text={result.content} isError={result.isError} />}
      </div>
    )
  }
  if (edits.length > 0) {
    return (
      <div className="space-y-2">
        {edits.map((e, i) => (
          <Diff key={i} before={e.before} after={e.after} />
        ))}
        {result?.isError && <Output text={result.content} isError />}
      </div>
    )
  }
  if (item.name === "Read") return result ? <Output text={result.content} isError={result.isError} /> : null
  if (item.name === "Task" || item.name === "Agent") {
    return (
      <div className="space-y-2">
        {typeof input.prompt === "string" && <p className="text-xs whitespace-pre-wrap text-muted-foreground">{input.prompt}</p>}
        {item.children.length > 0 && (
          <div className="space-y-2 border-l pl-3">
            {item.children.map((child) => (
              <ChildRow key={child.id} item={child} />
            ))}
          </div>
        )}
        {result && <Markdown text={result.content} />}
      </div>
    )
  }
  return (
    <div className="space-y-2">
      <CodeBlock code={JSON.stringify(item.input, null, 2)} language="json" />
      {result && <Output text={result.content} isError={result.isError} />}
    </div>
  )
}

function ChildRow({ item }: { item: ChatItem }) {
  if (item.kind === "tool") return <ToolCard item={item} />
  if (item.kind === "text") return <Markdown text={item.text} />
  return null
}

function PermissionBar({ request }: { request: PermissionRequest }) {
  const { respond } = useChatActions()
  if (request.allowed !== undefined) {
    return (
      <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {request.allowed ? <Check className="size-3.5" /> : <X className="size-3.5" />}
        {request.allowed ? "Allowed" : "Denied"}
      </p>
    )
  }
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="mr-auto flex items-center gap-1.5 text-xs text-amber-300">
        <ShieldAlert className="size-3.5" /> {request.description ? `Allow ${request.description}?` : "Allow this?"}
      </span>
      <Button size="sm" variant="ghost" className="h-7" onClick={() => respond(request.requestId, { allow: false, remember: false })}>
        Deny
      </Button>
      <Button size="sm" variant="outline" className="h-7" onClick={() => respond(request.requestId, { allow: true, remember: true })}>
        Always allow
      </Button>
      <Button size="sm" className="h-7" onClick={() => respond(request.requestId, { allow: true, remember: false })}>
        Allow
      </Button>
    </div>
  )
}

function QuestionForm({ request, questions }: { request: PermissionRequest; questions: Question[] }) {
  const { respond } = useChatActions()
  const [answers, setAnswers] = useState<Record<string, string[]>>({})
  const complete = questions.every((q) => (answers[q.question] ?? []).length > 0)
  const toggle = (q: Question, label: string) =>
    setAnswers((a) => {
      const current = a[q.question] ?? []
      if (!q.multiSelect) return { ...a, [q.question]: [label] }
      return { ...a, [q.question]: current.includes(label) ? current.filter((l) => l !== label) : [...current, label] }
    })

  if (request.allowed !== undefined) return null
  return (
    <div className="space-y-3">
      {questions.map((q) => (
        <div key={q.question} className="space-y-1.5">
          <p className="text-sm font-medium">{q.question}</p>
          <div className="grid gap-1.5">
            {q.options.map((o) => {
              const chosen = (answers[q.question] ?? []).includes(o.label)
              return (
                <button
                  key={o.label}
                  type="button"
                  onClick={() => toggle(q, o.label)}
                  className={cn(
                    "rounded-md border px-3 py-2 text-left text-sm transition-colors hover:bg-accent",
                    chosen && "border-primary bg-accent",
                  )}
                >
                  <span className="font-medium">{o.label}</span>
                  {o.description && <span className="block text-xs text-muted-foreground">{o.description}</span>}
                </button>
              )
            })}
          </div>
        </div>
      ))}
      <div className="flex justify-end gap-2">
        <Button size="sm" variant="ghost" className="h-7" onClick={() => respond(request.requestId, { allow: false, remember: false })}>
          Skip
        </Button>
        <Button
          size="sm"
          className="h-7"
          disabled={!complete}
          onClick={() =>
            respond(request.requestId, {
              allow: true,
              remember: false,
              updatedInput: {
                ...(request.input as object),
                answers: Object.fromEntries(Object.entries(answers).map(([q, labels]) => [q, labels.join(", ")])),
              },
            })
          }
        >
          Answer
        </Button>
      </div>
    </div>
  )
}

function PlanApproval({ request, plan }: { request: PermissionRequest; plan: string }) {
  const { respond, setMode } = useChatActions()
  return (
    <div className="space-y-3">
      <div className="max-h-96 overflow-auto rounded-md border bg-muted/20 p-3">
        <Markdown text={plan} />
      </div>
      {request.allowed === undefined && (
        <div className="flex flex-wrap justify-end gap-2">
          <Button
            size="sm"
            variant="ghost"
            className="h-7"
            onClick={() => respond(request.requestId, { allow: false, remember: false, message: "Keep planning; the user wants changes to the plan." })}
          >
            Keep planning
          </Button>
          <Button
            size="sm"
            variant="outline"
            className="h-7"
            onClick={() => {
              setMode("acceptEdits")
              respond(request.requestId, { allow: true, remember: false })
            }}
          >
            Approve and auto-accept edits
          </Button>
          <Button size="sm" className="h-7" onClick={() => respond(request.requestId, { allow: true, remember: false })}>
            Approve
          </Button>
        </div>
      )}
    </div>
  )
}

function Frame({ children, attention }: { children: ReactNode; attention?: boolean }) {
  return <div className={cn("rounded-lg border bg-card text-card-foreground", attention && "border-amber-500/50")}>{children}</div>
}

/** One tool call: a summary line that expands to its input and output. */
export const ToolCard = memo(function ToolCard({ item }: { item: ToolItem }) {
  const { cwd, openFile } = useChatActions()
  const summary = summarizeTool(item.name, item.input, cwd)
  const pending = !!item.permission && item.permission.allowed === undefined
  const [open, setOpen] = useState(false)
  const Icon = ICONS[item.name] ?? Wrench

  if (item.name === "TodoWrite") {
    return (
      <Frame>
        <div className="space-y-2 p-3">
          <p className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
            <ListTodo className="size-3.5" /> Plan
          </p>
          <Todos input={item.input} />
        </div>
      </Frame>
    )
  }

  const questions = item.name === "AskUserQuestion" ? questionsOf(item.input) : []
  if (questions.length > 0 && item.permission) {
    return (
      <Frame attention={pending}>
        <div className="p-3">
          {pending ? (
            <QuestionForm request={item.permission} questions={questions} />
          ) : (
            <p className="flex items-center gap-2 text-sm text-muted-foreground">
              <MessageCircleQuestion className="size-4" /> {item.result?.content ?? questions[0].question}
            </p>
          )}
        </div>
      </Frame>
    )
  }

  const plan = item.name === "ExitPlanMode" ? String((item.input as { plan?: unknown })?.plan ?? "") : ""
  if (plan && item.permission) {
    return (
      <Frame attention={pending}>
        <div className="space-y-2 p-3">
          <p className="text-xs font-medium text-muted-foreground">Plan ready for review</p>
          <PlanApproval request={item.permission} plan={plan} />
        </div>
      </Frame>
    )
  }

  const status = pending ? (
    <ShieldAlert className="size-3.5 text-amber-300" />
  ) : !item.result ? (
    <Loader2 className="size-3.5 animate-spin text-muted-foreground" />
  ) : item.result.isError ? (
    <X className="size-3.5 text-destructive" />
  ) : (
    <Check className="size-3.5 text-muted-foreground" />
  )

  return (
    <Frame attention={pending}>
      <Collapsible open={open || pending} onOpenChange={setOpen}>
        <CollapsibleTrigger className="group flex w-full min-w-0 items-center gap-2 px-3 py-2 text-left text-xs">
          <ChevronRight className="size-3.5 shrink-0 text-muted-foreground transition-transform group-data-[state=open]:rotate-90" />
          <Icon className="size-3.5 shrink-0 text-muted-foreground" />
          <span className="shrink-0 font-medium">{summary.title}</span>
          {summary.file ? (
            <span
              role="link"
              onClick={(e) => {
                e.stopPropagation()
                openFile(summary.file!)
              }}
              className="truncate font-mono text-muted-foreground hover:text-foreground hover:underline"
            >
              {summary.detail}
            </span>
          ) : (
            <span className="truncate font-mono text-muted-foreground">{summary.detail}</span>
          )}
          <span className="ml-auto shrink-0">{status}</span>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-2 px-3 pb-3">
          <Details item={item} />
          {item.permission && <PermissionBar request={item.permission} />}
        </CollapsibleContent>
      </Collapsible>
    </Frame>
  )
})

/** A permission request that didn't match a tool card. */
export function PermissionCard({ request }: { request: PermissionRequest }) {
  return (
    <Frame attention={request.allowed === undefined}>
      <div className="space-y-2 p-3">
        <p className="text-xs font-medium">{request.toolName}</p>
        <CodeBlock code={JSON.stringify(request.input, null, 2)} language="json" />
        <PermissionBar request={request} />
      </div>
    </Frame>
  )
}
