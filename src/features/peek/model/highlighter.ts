import { createHighlighter, type HighlighterGeneric, type ThemedToken } from "shiki"

const THEME = "github-dark-default"
const LANGS = ["typescript", "tsx", "javascript", "jsx"] as const

let highlighter: Promise<HighlighterGeneric<(typeof LANGS)[number], typeof THEME>> | null = null

function languageFor(path: string): (typeof LANGS)[number] {
  if (path.endsWith(".tsx")) return "tsx"
  if (path.endsWith(".jsx")) return "jsx"
  if (/\.[cm]?js$/.test(path)) return "javascript"
  return "typescript"
}

/** Syntax-highlighted lines of a file (lazily loads the grammars once). */
export async function highlightLines(path: string, code: string): Promise<ThemedToken[][]> {
  highlighter ??= createHighlighter({ themes: [THEME], langs: [...LANGS] })
  const h = await highlighter
  return h.codeToTokens(code, { lang: languageFor(path), theme: THEME }).tokens
}
