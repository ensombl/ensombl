/** How a tool call reads in the chat: a verb, its main argument, and the file it touches. */
export interface ToolSummary {
  title: string
  detail: string
  /** Repo-relative file, when the tool works on one. */
  file?: string
}

type Input = Record<string, unknown>

const str = (v: unknown) => (typeof v === "string" ? v : "")

/** Strips the worktree prefix so paths read like the rest of the app. */
export function relativePath(path: string, cwd: string): string {
  const root = cwd.endsWith("/") ? cwd : `${cwd}/`
  return path.startsWith(root) ? path.slice(root.length) : path
}

const firstLine = (text: string, max = 120) => {
  const line = text.split("\n")[0]
  return line.length > max ? `${line.slice(0, max)}…` : line
}

export function summarizeTool(name: string, raw: unknown, cwd: string): ToolSummary {
  const input = (raw && typeof raw === "object" ? raw : {}) as Input
  const file = str(input.file_path ?? input.notebook_path ?? input.path)
  const rel = file ? relativePath(file, cwd) : ""
  switch (name) {
    case "Bash":
      return { title: "Run", detail: firstLine(str(input.description) || str(input.command)) }
    case "Read":
      return { title: "Read", detail: rel, file: rel || undefined }
    case "Write":
      return { title: "Write", detail: rel, file: rel || undefined }
    case "Edit":
    case "MultiEdit":
    case "NotebookEdit":
      return { title: "Edit", detail: rel, file: rel || undefined }
    case "Glob":
      return { title: "Find files", detail: str(input.pattern) }
    case "Grep":
      return { title: "Search", detail: `${str(input.pattern)}${rel ? ` in ${rel}` : ""}` }
    case "LS":
      return { title: "List", detail: rel }
    case "WebFetch":
      return { title: "Fetch", detail: str(input.url) }
    case "WebSearch":
      return { title: "Web search", detail: str(input.query) }
    case "Task":
    case "Agent":
      return { title: str(input.subagent_type) || "Agent", detail: str(input.description) }
    case "TodoWrite":
      return { title: "Plan", detail: "" }
    case "ExitPlanMode":
      return { title: "Plan ready", detail: "" }
    case "AskUserQuestion":
      return { title: "Question", detail: "" }
    case "Skill":
      return { title: "Skill", detail: str(input.skill ?? input.command) }
    default: {
      const mcp = /^mcp__(.+?)__(.+)$/.exec(name)
      if (mcp) return { title: mcp[2].replace(/_/g, " "), detail: mcp[1] }
      return { title: name, detail: rel }
    }
  }
}

export interface DiffLine {
  op: " " | "-" | "+"
  text: string
}

const MAX_DIFF_CELLS = 250_000

/** Line diff by longest common subsequence; falls back to remove-all/add-all for huge inputs. */
export function diffLines(before: string, after: string): DiffLine[] {
  const a = before.split("\n")
  const b = after.split("\n")
  if (a.length * b.length > MAX_DIFF_CELLS) {
    return [...a.map((text) => ({ op: "-" as const, text })), ...b.map((text) => ({ op: "+" as const, text }))]
  }
  const lcs = Array.from({ length: a.length + 1 }, () => new Uint32Array(b.length + 1))
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      lcs[i][j] = a[i] === b[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1])
    }
  }
  const out: DiffLine[] = []
  let i = 0
  let j = 0
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      out.push({ op: " ", text: a[i] })
      i++
      j++
    } else if (lcs[i + 1][j] >= lcs[i][j + 1]) out.push({ op: "-", text: a[i++] })
    else out.push({ op: "+", text: b[j++] })
  }
  while (i < a.length) out.push({ op: "-", text: a[i++] })
  while (j < b.length) out.push({ op: "+", text: b[j++] })
  return out
}

/** The edits a file tool makes, as before/after pairs. */
export function fileEdits(name: string, raw: unknown): { before: string; after: string }[] {
  const input = (raw && typeof raw === "object" ? raw : {}) as Input
  if (name === "Edit") return [{ before: str(input.old_string), after: str(input.new_string) }]
  if (name === "MultiEdit" && Array.isArray(input.edits)) {
    return input.edits.map((e: Input) => ({ before: str(e.old_string), after: str(e.new_string) }))
  }
  if (name === "Write") return [{ before: "", after: str(input.content) }]
  return []
}

export interface Todo {
  content: string
  status: "pending" | "in_progress" | "completed"
}

export function todosOf(raw: unknown): Todo[] {
  const todos = (raw as { todos?: unknown })?.todos
  return Array.isArray(todos) ? todos.filter((t): t is Todo => typeof t?.content === "string") : []
}

export interface Question {
  question: string
  header?: string
  multiSelect?: boolean
  options: { label: string; description?: string }[]
}

export function questionsOf(raw: unknown): Question[] {
  const questions = (raw as { questions?: unknown })?.questions
  return Array.isArray(questions) ? questions.filter((q): q is Question => typeof q?.question === "string" && Array.isArray(q.options)) : []
}
