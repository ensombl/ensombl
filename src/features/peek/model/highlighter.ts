import { bundledLanguages, createHighlighter, type BundledLanguage, type HighlighterGeneric, type ThemedToken } from "shiki"

const THEME = "github-dark-default"
const LANGS = ["typescript", "tsx", "javascript", "jsx"] as const

let highlighter: Promise<HighlighterGeneric<BundledLanguage, typeof THEME>> | null = null

const getHighlighter = () => (highlighter ??= createHighlighter({ themes: [THEME], langs: [...LANGS] }))

function languageFor(path: string): (typeof LANGS)[number] {
  if (path.endsWith(".tsx")) return "tsx"
  if (path.endsWith(".jsx")) return "jsx"
  if (/\.[cm]?js$/.test(path)) return "javascript"
  return "typescript"
}

const isScript = (path: string) => /\.[cm]?[jt]sx?$/.test(path)

/** Syntax-highlighted lines of a file; other languages load on demand, unknown ones stay plain. */
export async function highlightLines(path: string, code: string): Promise<ThemedToken[][]> {
  const h = await getHighlighter()
  if (isScript(path)) return h.codeToTokens(code, { lang: languageFor(path), theme: THEME }).tokens
  const tokens = await highlightSnippet(code, languageOfPath(path))
  return tokens ?? code.split("\n").map((line) => [{ content: line, offset: 0 } as ThemedToken])
}

const EXTENSIONS: Record<string, string> = { ts: "typescript", js: "javascript", py: "python", rs: "rust", sh: "bash", md: "markdown", yml: "yaml" }

/** Highlighted lines for a snippet in any bundled language; `null` when the language is unknown. */
export async function highlightSnippet(code: string, language: string): Promise<ThemedToken[][] | null> {
  const lang = (EXTENSIONS[language] ?? language).toLowerCase()
  if (!(lang in bundledLanguages)) return null
  const h = await getHighlighter()
  if (!h.getLoadedLanguages().includes(lang)) await h.loadLanguage(lang as BundledLanguage)
  return h.codeToTokens(code, { lang: lang as BundledLanguage, theme: THEME }).tokens
}

/** Language id for a file path, for highlighting tool output. */
export function languageOfPath(path: string): string {
  const ext = path.split(".").pop() ?? ""
  return EXTENSIONS[ext] ?? ext
}
