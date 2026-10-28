import { ChevronsDownUp, ChevronsUpDown, Search, Shapes, SlidersHorizontal, Sparkle, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Toggle } from "@/components/ui/toggle"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import type { EdgeKind } from "@/shared/bindings/EdgeKind"

import { EDGE_STYLE } from "./edge-style"

const EDGE_KINDS: { kind: EdgeKind; label: string }[] = [
  { kind: "extends", label: "Extends" },
  { kind: "implements", label: "Implements" },
  { kind: "association", label: "Has" },
  { kind: "dependency", label: "Uses" },
]

interface DiagramToolbarProps {
  edgeKinds: EdgeKind[]
  hideTests: boolean
  showFunctions: boolean
  allGroupEdges: boolean
  onAllGroupEdges: (all: boolean) => void
  onSearch: () => void
  onEdgeKinds: (kinds: EdgeKind[]) => void
  onHideTests: (hide: boolean) => void
  onShowFunctions: (show: boolean) => void
  onExpandAll: () => void
  onCollapseAll: () => void
  changedCount: number
  onNextChange: () => void
  onClearChanges: () => void
  patternsOpen: boolean
  onTogglePatterns: () => void
}

function IconButton({ label, onClick, children }: { label: string; onClick: () => void; children: React.ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="ghost" size="icon" className="size-8 shrink-0" onClick={onClick} aria-label={label}>
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}

/** One compact row: secondary options live in the View menu so it never wraps over the map. */
export function DiagramToolbar(props: DiagramToolbarProps) {
  const keep = (e: Event) => e.preventDefault()
  return (
    <div className="flex max-w-full items-center gap-1 overflow-x-auto rounded-lg border bg-background p-1 shadow-sm">
      <Toggle size="sm" className="h-8 shrink-0 gap-1.5 text-xs" pressed={props.patternsOpen} onPressedChange={props.onTogglePatterns}>
        <Shapes /> Patterns
      </Toggle>
      <Button variant="ghost" size="sm" className="h-8 shrink-0 gap-1.5 text-xs text-muted-foreground" onClick={props.onSearch}>
        <Search /> Find
        <kbd className="rounded border bg-muted px-1 font-mono text-[10px]">⌘K</kbd>
      </Button>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="sm" className="h-8 shrink-0 gap-1.5 text-xs">
            <SlidersHorizontal /> View
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-60">
          <DropdownMenuLabel>Relations</DropdownMenuLabel>
          {EDGE_KINDS.map(({ kind, label }) => (
            <DropdownMenuCheckboxItem
              key={kind}
              checked={props.edgeKinds.includes(kind)}
              onCheckedChange={(on) => props.onEdgeKinds(on ? [...props.edgeKinds, kind] : props.edgeKinds.filter((k) => k !== kind))}
              onSelect={keep}
            >
              <svg width="16" height="6" className="mr-1 shrink-0">
                <line x1="0" y1="3" x2="16" y2="3" stroke={EDGE_STYLE[kind].color} strokeWidth="2" strokeDasharray={EDGE_STYLE[kind].dashed ? "4 3" : undefined} />
              </svg>
              {label}
            </DropdownMenuCheckboxItem>
          ))}
          <DropdownMenuSeparator />
          <DropdownMenuLabel>Show</DropdownMenuLabel>
          <DropdownMenuCheckboxItem checked={!props.hideTests} onCheckedChange={(on) => props.onHideTests(!on)} onSelect={keep}>
            Tests
          </DropdownMenuCheckboxItem>
          <DropdownMenuCheckboxItem checked={props.showFunctions} onCheckedChange={props.onShowFunctions} onSelect={keep}>
            Functions &amp; types
          </DropdownMenuCheckboxItem>
          <DropdownMenuCheckboxItem checked={props.allGroupEdges} onCheckedChange={props.onAllGroupEdges} onSelect={keep}>
            All group edges
          </DropdownMenuCheckboxItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <IconButton label="Expand one level" onClick={props.onExpandAll}>
        <ChevronsUpDown />
      </IconButton>
      <IconButton label="Collapse to packages" onClick={props.onCollapseAll}>
        <ChevronsDownUp />
      </IconButton>
      {props.changedCount > 0 && (
        <div className="flex h-8 shrink-0 items-center rounded-md border border-amber-500/40 bg-amber-500/10 text-amber-200">
          <button type="button" onClick={props.onNextChange} title="Jump to the latest change" className="flex h-full items-center gap-1.5 px-2.5 text-xs font-medium">
            <Sparkle className="size-3.5" />
            {props.changedCount} changed
          </button>
          <button
            type="button"
            onClick={props.onClearChanges}
            aria-label="Clear changes"
            className="flex h-full items-center border-l border-amber-500/30 px-1.5 hover:bg-amber-500/20"
          >
            <X className="size-3.5" />
          </button>
        </div>
      )}
    </div>
  )
}
