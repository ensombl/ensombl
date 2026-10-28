import { memo, useCallback, useMemo, useState } from "react"
import { ChevronRight, FileCode2, Folder, FolderOpen, Loader2, RefreshCw } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable"
import { cn } from "@/lib/utils"
import { PaneEmpty } from "@/shared/ui/pane"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { draftKey, useDraftsStore } from "../../model/drafts-store"
import { ancestorDirs, buildFileTree, searchFiles, type FileTreeNode } from "../../model/file-tree"
import { useSystemUi } from "../../model/system-store"
import { useFiles } from "../../queries/system-queries"
import { CodeEditor } from "./code-editor"

interface TreeProps {
  nodes: FileTreeNode[]
  depth: number
  open: Set<string>
  selected: string | null
  /** Files with unsaved edits. */
  dirty: (path: string) => boolean
  onToggle: (path: string) => void
  onSelect: (path: string) => void
}

const Tree = memo(function Tree({ nodes, depth, open, selected, dirty, onToggle, onSelect }: TreeProps) {
  return (
    <>
      {nodes.map((n) => {
        const isOpen = open.has(n.path)
        return (
          <div key={n.path}>
            <button
              type="button"
              onClick={() => (n.children ? onToggle(n.path) : onSelect(n.path))}
              className={cn(
                "flex h-6 w-full items-center gap-1 rounded-sm pr-2 text-left text-xs hover:bg-accent",
                n.path === selected && "bg-accent text-accent-foreground",
              )}
              style={{ paddingLeft: 6 + depth * 12 }}
              title={n.path}
            >
              {n.children ? (
                <>
                  <ChevronRight className={cn("size-3 shrink-0 text-muted-foreground transition-transform", isOpen && "rotate-90")} />
                  {isOpen ? <FolderOpen className="size-3.5 shrink-0 text-muted-foreground" /> : <Folder className="size-3.5 shrink-0 text-muted-foreground" />}
                </>
              ) : (
                <FileCode2 className="ml-4 size-3.5 shrink-0 text-muted-foreground" />
              )}
              <span className="truncate">{n.name}</span>
              {!n.children && dirty(n.path) && <span className="ml-auto size-1.5 shrink-0 rounded-full bg-foreground" title="Unsaved changes" />}
            </button>
            {n.children && isOpen && (
              <Tree nodes={n.children} depth={depth + 1} open={open} selected={selected} dirty={dirty} onToggle={onToggle} onSelect={onSelect} />
            )}
          </div>
        )
      })}
    </>
  )
})

/** The worktree's files: browse or filter, then edit one in place. */
export function CodeExplorer() {
  const worktree = useWorkspace()
  const ui = useSystemUi()
  const { file: selected, line } = ui.state
  // Listed the first time the tab is opened, not on every worktree load.
  const files = useFiles(worktree.id, ui.state.tab === "code")
  const dirty = useDraftsStore((s) => s.dirty)
  const isDirty = useCallback((path: string) => !!dirty[draftKey(worktree.id, path)], [dirty, worktree.id])
  const [query, setQuery] = useState("")
  const [open, setOpen] = useState<Set<string>>(() => new Set(selected ? ancestorDirs(selected) : []))
  const tree = useMemo(() => buildFileTree(files.data ?? []), [files.data])
  const matches = useMemo(() => searchFiles(files.data ?? [], query), [files.data, query])

  // Opened from elsewhere too (the component inspector): show where it lives.
  const [shown, setShown] = useState(selected)
  if (selected !== shown) {
    setShown(selected)
    if (selected) setOpen((prev) => new Set([...prev, ...ancestorDirs(selected)]))
  }

  const toggle = (path: string) =>
    setOpen((prev) => {
      const next = new Set(prev)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  const select = (path: string) => ui.openFile(path)

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full">
      <ResizablePanel id="file-tree" defaultSize="28" minSize="15" maxSize="60" className="flex flex-col">
        <div className="flex items-center gap-1 border-b p-2">
          <Input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Filter files…" className="h-8 text-xs" />
          <Button variant="ghost" size="icon" className="size-8 shrink-0" onClick={() => files.refetch()} aria-label="Refresh files">
            <RefreshCw className={cn(files.isFetching && "animate-spin")} />
          </Button>
        </div>
        <div className="min-h-0 flex-1 overflow-auto p-1">
          {files.isLoading && <Loader2 className="m-3 size-4 animate-spin text-muted-foreground" />}
          {query.trim() ? (
            matches.length === 0 ? (
              <p className="p-2 text-xs text-muted-foreground">No files match.</p>
            ) : (
              matches.map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => select(p)}
                  className={cn("flex w-full flex-col rounded-sm px-2 py-1 text-left hover:bg-accent", p === selected && "bg-accent")}
                >
                  <span className="truncate text-xs">{p.slice(p.lastIndexOf("/") + 1)}</span>
                  <span className="truncate font-mono text-[10px] text-muted-foreground">{p}</span>
                </button>
              ))
            )
          ) : (
            <Tree nodes={tree} depth={0} open={open} selected={selected} dirty={isDirty} onToggle={toggle} onSelect={select} />
          )}
        </div>
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel id="file-view" minSize="30" className="min-w-0">
        {selected ? (
          <CodeEditor key={selected} path={selected} line={line} onClose={() => ui.openFile(null)} />
        ) : (
          <PaneEmpty icon={<FileCode2 />} title="Pick a file" description={`${files.data?.length ?? 0} files in this worktree`} />
        )}
      </ResizablePanel>
    </ResizablePanelGroup>
  )
}
