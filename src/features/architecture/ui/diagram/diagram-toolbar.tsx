import { ChevronsDownUp, ChevronsUpDown, FlaskConical, Search } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import type { EdgeKind } from "@/shared/bindings/EdgeKind"

const EDGE_KINDS: { kind: EdgeKind; label: string; hint: string }[] = [
  { kind: "extends", label: "Extends", hint: "Generalization: solid line, hollow triangle" },
  { kind: "implements", label: "Implements", hint: "Realization: dashed line, hollow triangle" },
  { kind: "association", label: "Has", hint: "Association: fields and injected collaborators" },
  { kind: "dependency", label: "Uses", hint: "Dependency: types used in signatures (dashed)" },
]

interface DiagramToolbarProps {
  edgeKinds: EdgeKind[]
  hideTests: boolean
  onSearch: () => void
  onEdgeKinds: (kinds: EdgeKind[]) => void
  onHideTests: (hide: boolean) => void
  onExpandAll: () => void
  onCollapseAll: () => void
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
              <ToggleGroupItem value={kind} className="h-8 px-2.5 text-xs">
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
    </div>
  )
}
