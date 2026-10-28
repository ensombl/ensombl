import { useEffect, useRef, useState } from "react"
import { basicSetup } from "codemirror"
import { EditorState, type Text } from "@codemirror/state"
import { EditorView, keymap } from "@codemirror/view"
import { oneDark } from "@codemirror/theme-one-dark"
import { Code2, ExternalLink, Loader2, MessageSquare, Save, X } from "lucide-react"
import { toast } from "sonner"

import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { Pane, PaneEmpty } from "@/shared/ui/pane"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { useOpenInEditor, useSaveSource } from "@/features/peek/commands/peek-commands"
import { useComments, useSource } from "@/features/peek/queries/peek-queries"
import { CodeView } from "@/features/peek/ui/code-view"
import { EditorSettings } from "@/features/peek/ui/peek-panel"

import { draftKey, useDraftsStore } from "../../model/drafts-store"
import { languageFor } from "../../model/editor-languages"

const theme = EditorView.theme(
  {
    "&": { height: "100%", fontSize: "12.5px", backgroundColor: "#0d1117" },
    ".cm-gutters": { backgroundColor: "#0d1117", border: "none" },
    ".cm-scroller": { fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", lineHeight: "1.6" },
    "&.cm-focused": { outline: "none" },
  },
  { dark: true },
)

interface EditorProps {
  worktreeId: string
  path: string
  /** The file on disk now. */
  content: string
  line: number
  onDirty: (dirty: boolean) => void
  onConflict: (conflict: boolean) => void
  /** Saves are triggered from the header and ⌘S. */
  saveRef: React.RefObject<((overwrite?: boolean) => void) | null>
  reloadRef: React.RefObject<(() => void) | null>
}

function Editor({ worktreeId, path, content, line, onDirty, onConflict, saveRef, reloadRef }: EditorProps) {
  const host = useRef<HTMLDivElement>(null)
  const view = useRef<EditorView | null>(null)
  // What the editor's text is based on: the disk content it last loaded or saved.
  const base = useRef<{ content: string; text: Text | null }>({ content, text: null })
  const dirty = useRef(false)
  const save = useSaveSource()
  const key = draftKey(worktreeId, path)

  const setDirty = (value: boolean) => {
    dirty.current = value
    onDirty(value)
  }
  const replace = (text: string) => {
    const v = view.current!
    base.current = { content: text, text: null }
    v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert: text } })
    base.current.text = v.state.doc
    setDirty(false)
    onConflict(false)
  }

  const saveNow = (overwrite = false) => {
    const v = view.current
    if (!v) return
    const text = v.state.doc
    const next = text.toString()
    save.mutate(
      { worktreeId, path, content: next, base: overwrite ? undefined : base.current.content },
      {
        onSuccess: () => {
          base.current = { content: next, text }
          setDirty(!v.state.doc.eq(text))
          onConflict(false)
        },
        onError: (error) => {
          if (String(error).includes("changed on disk")) onConflict(true)
          else toast.error("Couldn't save", { description: String(error) })
        },
      },
    )
  }
  // The header's buttons and ⌘S reach the latest closures through these.
  useEffect(() => {
    saveRef.current = saveNow
    reloadRef.current = () => replace(content)
  })

  // One editor per file; a kept draft picks up where it left off.
  useEffect(() => {
    const draft = useDraftsStore.getState().drafts[key]
    base.current = { content: draft?.base ?? content, text: null }
    const state = EditorState.create({
      doc: draft?.content ?? content,
      extensions: [
        basicSetup,
        languageFor(path),
        oneDark,
        theme,
        keymap.of([{ key: "Mod-s", preventDefault: true, run: () => (saveRef.current?.(), true) }]),
        EditorView.updateListener.of((u) => {
          if (!u.docChanged) return
          const now = !!base.current.text && !u.state.doc.eq(base.current.text)
          if (now !== dirty.current) setDirty(now)
        }),
      ],
    })
    const v = new EditorView({ state, parent: host.current! })
    view.current = v
    base.current.text = draft ? EditorState.create({ doc: draft.base }).doc : v.state.doc
    if (draft) setDirty(!v.state.doc.eq(base.current.text))
    return () => {
      const store = useDraftsStore.getState()
      store.keep(key, dirty.current ? { content: v.state.doc.toString(), base: base.current.content } : null)
      v.destroy()
      view.current = null
    }
    // The file's first content only; later disk changes are merged below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key])

  // The file changed on disk (usually an agent): follow it unless you've edited.
  useEffect(() => {
    if (!view.current || content === base.current.content) return
    if (dirty.current) onConflict(true)
    else replace(content)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [content])

  useEffect(() => {
    const v = view.current
    if (!v) return
    const at = v.state.doc.line(Math.min(Math.max(line, 1), v.state.doc.lines)).from
    v.dispatch({ selection: { anchor: at }, effects: EditorView.scrollIntoView(at, { y: "center" }) })
  }, [line])

  return <div ref={host} className="h-full overflow-hidden" />
}

/** A worktree file, editable in place; ⌘S saves. Comments live in review mode. */
export function CodeEditor({ path, line, onClose }: { path: string; line: number; onClose: () => void }) {
  const worktree = useWorkspace()
  const source = useSource(worktree.id, path)
  const comments = useComments(worktree.id)
  const openInEditor = useOpenInEditor()
  const key = draftKey(worktree.id, path)
  const dirty = useDraftsStore((s) => !!s.dirty[key])
  const setDirty = useDraftsStore((s) => s.setDirty)
  const [conflict, setConflict] = useState(false)
  const [review, setReview] = useState(false)
  const saveRef = useRef<((overwrite?: boolean) => void) | null>(null)
  const reloadRef = useRef<(() => void) | null>(null)
  const title = path.slice(path.lastIndexOf("/") + 1)
  const fileComments = (comments.data ?? []).filter((c) => c.comment.file === path)

  return (
    <Pane
      title={dirty ? `${title} •` : title}
      icon={<Code2 />}
      actions={
        <>
          <span className="truncate font-mono text-xs text-muted-foreground">{path}</span>
          <Button size="sm" variant="ghost" className="h-7 gap-1.5 text-xs" disabled={!dirty} onClick={() => saveRef.current?.()}>
            <Save /> Save
          </Button>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant={review ? "secondary" : "ghost"}
                size="icon"
                className="size-7"
                aria-label="Review comments"
                onClick={() => setReview((r) => !r)}
              >
                <MessageSquare />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{review ? "Back to editing" : "Comment on lines"}</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="size-7"
                aria-label="Open in editor"
                onClick={() => openInEditor.mutate({ worktreeId: worktree.id, path, line })}
              >
                <ExternalLink />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Open in editor</TooltipContent>
          </Tooltip>
          <EditorSettings />
          <Button variant="ghost" size="icon" className="size-7" onClick={onClose} aria-label="Close file">
            <X />
          </Button>
        </>
      }
    >
      {!source.data ? (
        <PaneEmpty
          icon={source.isError ? <Code2 /> : <Loader2 className="animate-spin" />}
          title={source.isError ? "Couldn't read file" : "Loading…"}
          description={source.isError ? String(source.error) : path}
        />
      ) : (
        <div className="flex h-full flex-col">
          {conflict && (
            <div className="flex shrink-0 items-center gap-2 border-b bg-amber-400/10 px-3 py-1.5 text-xs">
              <span className="text-amber-200">This file changed on disk since you started editing.</span>
              <Button size="sm" variant="outline" className="ml-auto h-6 text-xs" onClick={() => reloadRef.current?.()}>
                Load theirs
              </Button>
              <Button size="sm" variant="outline" className="h-6 text-xs" onClick={() => saveRef.current?.(true)}>
                Keep mine
              </Button>
            </div>
          )}
          <div className="min-h-0 flex-1">
            {/* Review mode shows the file on disk; the editor stays mounted so edits survive. */}
            {review && <CodeView path={path} content={source.data.content} line={line} comments={fileComments} />}
            <div className={review ? "hidden" : "h-full"}>
              <Editor
                worktreeId={worktree.id}
                path={path}
                content={source.data.content}
                line={line}
                onDirty={(d) => setDirty(key, d)}
                onConflict={setConflict}
                saveRef={saveRef}
                reloadRef={reloadRef}
              />
            </div>
          </div>
        </div>
      )}
    </Pane>
  )
}
