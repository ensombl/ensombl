import { Fragment, useEffect, useMemo, useRef, useState } from "react"
import type { ThemedToken } from "shiki"
import { Plus } from "lucide-react"

import { cn } from "@/lib/utils"
import type { AnchoredComment } from "@/shared/bindings/AnchoredComment"

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

/** Read-only, syntax-highlighted code with GitHub-style line comments. */
export function CodeView({ path, content, line, comments }: CodeViewProps) {
  const [tokens, setTokens] = useState<ThemedToken[][] | null>(null)
  const [composing, setComposing] = useState<number | null>(null)
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

  const byLine = useMemo(() => {
    const map = new Map<number, AnchoredComment[]>()
    for (const c of comments) {
      if (c.currentLine === null) continue
      map.set(c.currentLine, [...(map.get(c.currentLine) ?? []), c])
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
          return (
            <Fragment key={n}>
              <div
                ref={n === line ? target : undefined}
                className={cn("group flex hover:bg-white/[0.03]", n === line && "bg-amber-400/10 hover:bg-amber-400/15")}
              >
                <div className="relative w-14 shrink-0 pr-3 text-right text-zinc-600 select-none group-hover:text-zinc-400">
                  <button
                    type="button"
                    aria-label={`Comment on line ${n}`}
                    onClick={() => setComposing(n)}
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
              {(threads || composing === n) && (
                <div className="sticky left-0 max-w-2xl space-y-2 py-2 pr-4 pl-14 font-sans">
                  {threads?.map((c) => (
                    <CommentCard key={c.comment.id} item={c} />
                  ))}
                  {composing === n && <CommentForm file={path} line={n} onDone={() => setComposing(null)} />}
                </div>
              )}
            </Fragment>
          )
        })}
      </div>
    </div>
  )
}
