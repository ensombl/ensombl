import { ChevronsDownUp, ChevronsUpDown, FlaskConical, FunctionSquare, Network, Search, Sparkle, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import type { EdgeKind } from "@/shared/bindings/EdgeKind"

import { EDGE_STYLE } from "./edge-style"

const EDGE_KINDS: { kind: EdgeKind; label: string; hint: string }[] = [
  { kind: "extends", label: "Extends", hint: "Generalization: solid line, hollow triangle" },
  { kind: "implements", label: "Implements", hint: "Realization: dashed line, hollow triangle" },
  { kind: "association", label: "Has", hint: "Association: fields and injected collaborators" },
  { kind: "dependency", label: "Uses", hint: "Dependency: types used in signatures (dashed)" },
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
}

export function DiagramToolbar(props: DiagramToolbarProps) {
  return (
    <div className="flex flex-wrap items-center gap-2 rounded-lg border bg-background/90 p-1.5 shadow-sm backdrop-blur">
      <Button variant="outline" size="sm" className="h-8 gap-2 text-muted-foreground" onClick={props.onSearch}>
        <Search /> Find symbol
        <kbd className="rounded border bg-muted px-1 font-mono text-[10px]">⌘K</kbd>
      </Button>
      <ToggleGroup
        type="multiple"
        variant="outline"
        size="sm"
        value={props.edgeKinds}
        onValueChange={(v) => props.onEdgeKinds(v as EdgeKind[])}
      >
        {EDGE_KINDS.map(({ kind, label, hint }) => (
          <Tooltip key={kind}>
            <TooltipTrigger asChild>
              <ToggleGroupItem value={kind} className="h-8 gap-1.5 px-2.5 text-xs">
                <svg width="16" height="6" className="shrink-0">
                  <line
                    x1="0"
                    y1="3"
                    x2="16"
                    y2="3"
                    stroke={EDGE_STYLE[kind].color}
                    strokeWidth="2"
                    strokeDasharray={EDGE_STYLE[kind].dashed ? "4 3" : undefined}
                  />
                </svg>
                {label}
              </ToggleGroupItem>
            </TooltipTrigger>
            <TooltipContent>{hint}</TooltipContent>
          </Tooltip>
        ))}
      </ToggleGroup>
      <Label className="flex h-8 items-center gap-2 rounded-md border px-2.5 text-xs font-normal">
        <FlaskConical className="size-3.5 text-muted-foreground" />
        Hide tests
        <Switch checked={props.hideTests} onCheckedChange={props.onHideTests} />
      </Label>
      <Label className="flex h-8 items-center gap-2 rounded-md border px-2.5 text-xs font-normal">
        <FunctionSquare className="size-3.5 text-muted-foreground" />
        Functions &amp; types
        <Switch checked={props.showFunctions} onCheckedChange={props.onShowFunctions} />
      </Label>
      <Tooltip>
        <TooltipTrigger asChild>
          <Label className="flex h-8 items-center gap-2 rounded-md border px-2.5 text-xs font-normal">
            <Network className="size-3.5 text-muted-foreground" />
            All group edges
            <Switch checked={props.allGroupEdges} onCheckedChange={props.onAllGroupEdges} />
          </Label>
        </TooltipTrigger>
        <TooltipContent>Off: only the selected package/folder shows its dependencies</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button variant="ghost" size="icon" className="size-8" onClick={props.onExpandAll} aria-label="Expand one level">
            <ChevronsUpDown />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Expand one level</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button variant="ghost" size="icon" className="size-8" onClick={props.onCollapseAll} aria-label="Collapse all">
            <ChevronsDownUp />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Collapse to packages</TooltipContent>
      </Tooltip>
      {props.changedCount > 0 && (
        <div className="flex h-8 items-center rounded-md border border-amber-500/40 bg-amber-500/10 text-amber-200">
          <Tooltip>
            <TooltipTrigger asChild>
              <button type="button" onClick={props.onNextChange} className="flex h-full items-center gap-1.5 px-2.5 text-xs font-medium">
                <Sparkle className="size-3.5" />
                {props.changedCount} changed
              </button>
            </TooltipTrigger>
            <TooltipContent>Jump to the latest change</TooltipContent>
          </Tooltip>
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
