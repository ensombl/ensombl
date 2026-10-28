import { memo, useEffect, useState, type ComponentProps } from "react"
import ReactMarkdown, { type Components } from "react-markdown"
import remarkGfm from "remark-gfm"
import { openUrl } from "@tauri-apps/plugin-opener"
import type { ThemedToken } from "shiki"

import { cn } from "@/lib/utils"
import { highlightSnippet } from "@/features/peek/model/highlighter"

/** A fenced code block, highlighted once the grammar loads. */
export function CodeBlock({ code, language, className }: { code: string; language: string; className?: string }) {
  const [tokens, setTokens] = useState<ThemedToken[][] | null>(null)
  useEffect(() => {
    let cancelled = false
    highlightSnippet(code, language)
      .then((t) => !cancelled && setTokens(t))
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [code, language])

  return (
    <pre className={cn("overflow-x-auto rounded-md border bg-muted/40 p-3 font-mono text-xs leading-relaxed", className)}>
      <code>
        {tokens
          ? tokens.map((line, i) => (
              <div key={i}>
                {line.map((t, j) => (
                  <span key={j} style={{ color: t.color }}>
                    {t.content}
                  </span>
                ))}
                {line.length === 0 && "\n"}
              </div>
            ))
          : code}
      </code>
    </pre>
  )
}

const components: Components = {
  p: (props) => <p className="leading-relaxed [&:not(:first-child)]:mt-3" {...props} />,
  a: ({ href, children }) => (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault()
        if (href) void openUrl(href)
      }}
      className="font-medium text-primary underline underline-offset-4"
    >
      {children}
    </a>
  ),
  h1: (props) => <h3 className="mt-4 mb-2 text-base font-semibold first:mt-0" {...props} />,
  h2: (props) => <h3 className="mt-4 mb-2 text-sm font-semibold first:mt-0" {...props} />,
  h3: (props) => <h4 className="mt-3 mb-1.5 text-sm font-semibold first:mt-0" {...props} />,
  h4: (props) => <h4 className="mt-3 mb-1 text-sm font-medium first:mt-0" {...props} />,
  ul: (props) => <ul className="my-2 ml-5 list-disc space-y-1 [&_ul]:my-1" {...props} />,
  ol: (props) => <ol className="my-2 ml-5 list-decimal space-y-1 [&_ol]:my-1" {...props} />,
  li: (props) => <li className="leading-relaxed" {...props} />,
  blockquote: (props) => <blockquote className="my-2 border-l-2 pl-3 text-muted-foreground" {...props} />,
  hr: () => <hr className="my-4 border-border" />,
  table: (props) => (
    <div className="my-3 overflow-x-auto rounded-md border">
      <table className="w-full text-xs" {...props} />
    </div>
  ),
  th: (props) => <th className="border-b bg-muted/50 px-2 py-1.5 text-left font-medium" {...props} />,
  td: (props) => <td className="border-b px-2 py-1.5 align-top last:border-b-0" {...props} />,
  pre: ({ children }) => <>{children}</>,
  code: ({ className, children, ...props }: ComponentProps<"code">) => {
    const text = String(children ?? "")
    const language = /language-([\w+-]+)/.exec(className ?? "")?.[1]
    if (language || text.includes("\n")) {
      return <CodeBlock code={text.replace(/\n$/, "")} language={language ?? "text"} className="my-3" />
    }
    return (
      <code className="rounded bg-muted px-1 py-0.5 font-mono text-[0.85em]" {...props}>
        {children}
      </code>
    )
  },
}

export const Markdown = memo(function Markdown({ text }: { text: string }) {
  return (
    <div className="text-sm break-words">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {text}
      </ReactMarkdown>
    </div>
  )
})
