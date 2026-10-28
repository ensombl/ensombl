import { Fragment, useEffect, useMemo, useRef, useState } from "react"
import type { ThemedToken } from "shiki"
import { Plus } from "lucide-react"

import { cn } from "@/lib/utils"
import type { AnchoredComment } from "@/shared/bindings/AnchoredComment"

import { currentRange, ordered } from "../model/comment-range"
import { highlightLines } from "../model/highlighter"
import { CommentCard, CommentForm } from "./comment-thread"

interface CodeViewProps {
  path: string
  content: string
  /** Line to scroll to and highlight. */
  line: number
  comments: AnchoredComment[]
}

const MAX_LINES = 8000

/** Read-only, syntax-highlighted code with GitHub-style line and range comments. */
export function CodeView({ path, content, line, comments }: CodeViewProps) {
  const [tokens, setTokens] = useState<ThemedToken[][] | null>(null)
  // Press on a line number and drag to comment on a range, like GitHub.
  const [drag, setDrag] = useState<{ from: number; to: number } | null>(null)
  const [composing, setComposing] = useState<{ start: number; end: number } | null>(null)
  const picked = drag ? ordered(drag.from, drag.to) : composing
  const target = useRef<HTMLDivElement>(null)
  const lines = useMemo(() => content.split("\n").slice(0, MAX_LINES), [content])

  useEffect(() => {
    let cancelled = false
    highlightLines(path, lines.join("\n")).then((t) => !cancelled && setTokens(t))
    return () => {
      cancelled = true
    }
  }, [path, lines])

  useEffect(() => {
    target.current?.scrollIntoView({ block: "center" })
  }, [line, tokens])

  useEffect(() => {
    if (!drag) return
    const finish = () => {
      setComposing(ordered(drag.from, drag.to))
      setDrag(null)
    }
    window.addEventListener("mouseup", finish, { once: true })
    return () => window.removeEventListener("mouseup", finish)
  }, [drag])

  // Threads sit under the last line they cover.
  const byLine = useMemo(() => {
    const map = new Map<number, AnchoredComment[]>()
    for (const c of comments) {
      const range = currentRange(c)
      if (!range) continue
      map.set(range.end, [...(map.get(range.end) ?? []), c])
    }
    return map
  }, [comments])
  const outdated = comments.filter((c) => c.currentLine === null)

  return (
    <div className="h-full overflow-auto bg-[#0d1117] font-mono text-[12.5px] leading-5">
      {outdated.length > 0 && (
        <div className="space-y-2 border-b p-3 font-sans">
          <p className="text-xs font-medium text-muted-foreground">Comments on code that has changed</p>
          {outdated.map((c) => (
            <CommentCard key={c.comment.id} item={c} />
          ))}
        </div>
      )}
      <div className="min-w-max py-2">
        {lines.map((text, i) => {
          const n = i + 1
          const threads = byLine.get(n)
          const inPick = !!picked && n >= picked.start && n <= picked.end
          return (
            <Fragment key={n}>
              <div
                ref={n === line ? target : undefined}
                onMouseEnter={() => drag && setDrag({ ...drag, to: n })}
                className={cn(
                  "group flex hover:bg-white/[0.03]",
                  n === line && "bg-amber-400/10 hover:bg-amber-400/15",
                  inPick && "bg-blue-500/15 hover:bg-blue-500/15",
                  drag && "select-none",
                )}
              >
                <div
                  onMouseDown={(e) => {
                    if (e.button !== 0) return
                    e.preventDefault()
                    setDrag({ from: n, to: n })
                  }}
                  className={cn(
                    "relative w-14 shrink-0 cursor-pointer pr-3 text-right text-zinc-600 select-none group-hover:text-zinc-400",
                    inPick && "text-blue-300",
                  )}
                >
                  <button
                    type="button"
                    aria-label={`Comment on line ${n}`}
                    className="absolute top-0.5 left-1 hidden size-4 items-center justify-center rounded bg-blue-500 text-white group-hover:flex"
                  >
                    <Plus className="size-3" />
                  </button>
                  {n}
                </div>
                <pre className="pr-6 whitespace-pre">
                  {tokens?.[i]?.map((t, j) => (
                    <span key={j} style={{ color: t.color }}>
                      {t.content}
                    </span>
                  )) ?? <span className="text-zinc-300">{text}</span>}
                </pre>
              </div>
              {(threads || composing?.end === n) && (
                <div className="sticky left-0 max-w-2xl space-y-2 py-2 pr-4 pl-14 font-sans">
                  {threads?.map((c) => (
                    <CommentCard key={c.comment.id} item={c} />
                  ))}
                  {composing?.end === n && (
                    <CommentForm file={path} line={composing.start} endLine={composing.end} onDone={() => setComposing(null)} />
                  )}
                </div>
              )}
            </Fragment>
          )
        })}
      </div>
    </div>
  )
}
